use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const APP_NAME: &str = "jumper";
pub const CONFIG_DIR_NAME: &str = ".x-cli-jumper";
pub const CONFIG_FILE_NAME: &str = "config.toml";

const CONFIG_VERSION: u32 = 1;

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    "vendor",
    "venv",
    ".venv",
    "__pycache__",
    ".tox",
    ".cache",
    ".cargo",
    ".rustup",
    ".npm",
    ".pnpm-store",
    ".gradle",
    ".m2",
    "Library",
    "Applications",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sector {
    pub label: String,
    pub name: String,
    pub above: String,
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectConfig {
    pub version: u32,
    pub projects: Vec<ProjectConfigEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectConfigEntry {
    pub path: PathBuf,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    pub sector_index: usize,
    pub project_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoiceParseError {
    Empty,
    MissingSector,
    MissingNumber,
    InvalidNumber,
    InvalidSector,
    ZeroPosition,
}

impl ChoiceParseError {
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Empty => "empty input cancels",
            Self::MissingSector => "expected a sector label, for example A1",
            Self::MissingNumber => "expected a number after the sector label",
            Self::InvalidNumber => "expected a positive number after the sector label",
            Self::InvalidSector => "sector label is too large",
            Self::ZeroPosition => "project position starts at 1",
        }
    }
}

pub fn discover_projects(root: &Path) -> io::Result<Vec<PathBuf>> {
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} is not a directory", root.display()),
        ));
    }

    let mut projects = Vec::new();
    scan_dir(root, &mut projects, true)?;
    projects.sort();
    projects.dedup();
    Ok(projects)
}

#[must_use]
pub fn config_path(home: &Path) -> PathBuf {
    home.join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME)
}

pub fn load_project_config(path: &Path) -> io::Result<ProjectConfig> {
    let contents = fs::read_to_string(path)?;
    parse_project_config(&contents)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidData, message))
}

pub fn write_project_config(path: &Path, config: &ProjectConfig) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, render_project_config(config))
}

#[must_use]
pub fn merge_project_config(
    existing: Option<ProjectConfig>,
    discovered: Vec<PathBuf>,
) -> ProjectConfig {
    let mut projects = BTreeMap::new();

    if let Some(existing) = existing {
        for project in existing.projects {
            projects.insert(project.path, project.active);
        }
    }

    for path in discovered {
        projects.entry(path).or_insert(true);
    }

    ProjectConfig {
        version: CONFIG_VERSION,
        projects: projects
            .into_iter()
            .map(|(path, active)| ProjectConfigEntry { path, active })
            .collect(),
    }
}

#[must_use]
pub fn active_project_paths(config: &ProjectConfig) -> Vec<PathBuf> {
    let mut paths = BTreeSet::new();

    for project in &config.projects {
        if project.active && project.path.join(".git").exists() {
            paths.insert(project.path.clone());
        }
    }

    paths.into_iter().collect()
}

pub fn parse_project_config(contents: &str) -> Result<ProjectConfig, String> {
    let mut version = None;
    let mut projects = Vec::new();
    let mut current_project: Option<ProjectConfigEntryDraft> = None;

    for (line_index, raw_line) in contents.lines().enumerate() {
        let line_number = line_index + 1;
        let line = strip_toml_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if line == "[[projects]]" {
            if let Some(project) = current_project.take() {
                finish_project_config_entry(project, &mut projects, line_number)?;
            }
            current_project = Some(ProjectConfigEntryDraft::default());
            continue;
        }

        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!("line {line_number}: expected key = value"));
        };
        let key = raw_key.trim();
        let value = raw_value.trim();

        match current_project.as_mut() {
            Some(project) => match key {
                "path" => {
                    project.path = Some(PathBuf::from(parse_toml_string(value, line_number)?))
                }
                "active" => project.active = Some(parse_toml_bool(value, line_number)?),
                _ => return Err(format!("line {line_number}: unknown project key `{key}`")),
            },
            None => match key {
                "version" => version = Some(parse_config_version(value, line_number)?),
                _ => return Err(format!("line {line_number}: unknown config key `{key}`")),
            },
        }
    }

    if let Some(project) = current_project {
        finish_project_config_entry(project, &mut projects, contents.lines().count() + 1)?;
    }

    let version = version.unwrap_or(CONFIG_VERSION);
    if version != CONFIG_VERSION {
        return Err(format!("unsupported config version {version}"));
    }

    Ok(ProjectConfig { version, projects })
}

#[must_use]
pub fn render_project_config(config: &ProjectConfig) -> String {
    let mut output = String::from(
        "# jumper project config\n# Set active = false to hide a project.\n\nversion = 1\n",
    );

    for project in &config.projects {
        output.push_str("\n[[projects]]\npath = \"");
        push_toml_string(&mut output, &project.path.display().to_string());
        output.push_str("\"\nactive = ");
        output.push_str(if project.active { "true" } else { "false" });
        output.push('\n');
    }

    output
}

pub fn group_projects(scan_root: &Path, projects: Vec<PathBuf>) -> Vec<Sector> {
    let mut grouped: BTreeMap<(String, String), Vec<PathBuf>> = BTreeMap::new();

    for project in projects {
        let parent = project.parent();
        let sector = parent
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Misc")
            .to_owned();
        let above = parent
            .and_then(Path::parent)
            .map(|path| display_relative_root(scan_root, path))
            .unwrap_or_else(|| "/".to_owned());

        grouped.entry((sector, above)).or_default().push(project);
    }

    grouped
        .into_iter()
        .enumerate()
        .map(|(index, ((name, above), mut paths))| {
            paths.sort();
            Sector {
                label: label_for_index(index),
                name,
                above,
                paths,
            }
        })
        .collect()
}

pub fn parse_choice(input: &str) -> Result<Choice, ChoiceParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ChoiceParseError::Empty);
    }

    let split = trimmed
        .find(|ch: char| !ch.is_ascii_alphabetic())
        .unwrap_or(trimmed.len());
    if split == 0 {
        return Err(ChoiceParseError::MissingSector);
    }

    let label = &trimmed[..split];
    let number = trimmed[split..].trim();
    if number.is_empty() {
        return Err(ChoiceParseError::MissingNumber);
    }
    if !number.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(ChoiceParseError::InvalidNumber);
    }

    let project_number = number
        .parse::<usize>()
        .map_err(|_| ChoiceParseError::InvalidNumber)?;
    if project_number == 0 {
        return Err(ChoiceParseError::ZeroPosition);
    }

    let sector_index = index_for_label(label).ok_or(ChoiceParseError::InvalidSector)?;

    Ok(Choice {
        sector_index,
        project_index: project_number - 1,
    })
}

#[must_use]
pub fn label_for_index(mut index: usize) -> String {
    let mut bytes = Vec::new();

    loop {
        let remainder = index % 26;
        bytes.push(b'A' + remainder as u8);
        if index < 26 {
            break;
        }
        index = (index / 26) - 1;
    }

    bytes.reverse();
    String::from_utf8(bytes).expect("label is always ASCII")
}

fn index_for_label(label: &str) -> Option<usize> {
    let mut value = 0usize;

    for byte in label.bytes() {
        if !byte.is_ascii_alphabetic() {
            return None;
        }
        let letter = byte.to_ascii_uppercase() - b'A' + 1;
        value = value.checked_mul(26)?.checked_add(letter as usize)?;
    }

    value.checked_sub(1)
}

#[derive(Default)]
struct ProjectConfigEntryDraft {
    path: Option<PathBuf>,
    active: Option<bool>,
}

fn finish_project_config_entry(
    project: ProjectConfigEntryDraft,
    projects: &mut Vec<ProjectConfigEntry>,
    line_number: usize,
) -> Result<(), String> {
    let path = project
        .path
        .ok_or_else(|| format!("line {line_number}: project entry is missing path"))?;

    projects.push(ProjectConfigEntry {
        path,
        active: project.active.unwrap_or(true),
    });
    Ok(())
}

fn parse_config_version(value: &str, line_number: usize) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("line {line_number}: version must be an integer"))
}

fn parse_toml_bool(value: &str, line_number: usize) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("line {line_number}: active must be true or false")),
    }
}

fn parse_toml_string(value: &str, line_number: usize) -> Result<String, String> {
    if !value.starts_with('"') || !value.ends_with('"') || value.len() < 2 {
        return Err(format!("line {line_number}: path must be a quoted string"));
    }

    let mut chars = value[1..value.len() - 1].chars();
    let mut output = String::new();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }

        let Some(escaped) = chars.next() else {
            return Err(format!("line {line_number}: unfinished escape sequence"));
        };

        match escaped {
            '"' => output.push('"'),
            '\\' => output.push('\\'),
            'n' => output.push('\n'),
            'r' => output.push('\r'),
            't' => output.push('\t'),
            'u' => output.push(parse_unicode_escape(&mut chars, 4, line_number)?),
            'U' => output.push(parse_unicode_escape(&mut chars, 8, line_number)?),
            _ => {
                return Err(format!(
                    "line {line_number}: unsupported escape `\\{escaped}`"
                ));
            }
        }
    }

    Ok(output)
}

fn parse_unicode_escape(
    chars: &mut std::str::Chars<'_>,
    digits: usize,
    line_number: usize,
) -> Result<char, String> {
    let mut value = 0u32;

    for _ in 0..digits {
        let Some(ch) = chars.next() else {
            return Err(format!("line {line_number}: unfinished unicode escape"));
        };
        let Some(digit) = ch.to_digit(16) else {
            return Err(format!("line {line_number}: invalid unicode escape"));
        };
        value = (value << 4) + digit;
    }

    char::from_u32(value).ok_or_else(|| format!("line {line_number}: invalid unicode scalar"))
}

fn strip_toml_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in line.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }

            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '#' => return &line[..index],
            _ => {}
        }
    }

    line
}

fn push_toml_string(output: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch.is_control() => output.push_str(&format!("\\u{:04X}", ch as u32)),
            ch => output.push(ch),
        }
    }
}

fn scan_dir(root: &Path, out: &mut Vec<PathBuf>, is_root: bool) -> io::Result<()> {
    if !is_root && root.join(".git").exists() {
        out.push(root.to_path_buf());
        return Ok(());
    }

    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if is_root => return Err(error),
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }

        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if should_skip_dir(name) {
            continue;
        }

        scan_dir(&path, out, false)?;
    }

    Ok(())
}

fn should_skip_dir(name: &str) -> bool {
    name.starts_with('.') || SKIP_DIRS.contains(&name)
}

fn display_relative_root(scan_root: &Path, path: &Path) -> String {
    match path.strip_prefix(scan_root) {
        Ok(relative) if relative.as_os_str().is_empty() => "~".to_owned(),
        Ok(relative) => format!("~/{}", relative.display()),
        Err(_) => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn scans_git_projects_and_skips_noise() {
        let root = temp_root("scan");
        let project = root.join("work/apps/jumper");
        let nested = project.join("nested/ignored");
        let skipped = root.join("node_modules/package");
        let hidden = root.join(".config/project");

        fs::create_dir_all(project.join(".git")).expect("create project git dir");
        fs::create_dir_all(nested.join(".git")).expect("create nested git dir");
        fs::create_dir_all(skipped.join(".git")).expect("create skipped git dir");
        fs::create_dir_all(hidden.join(".git")).expect("create hidden git dir");

        let projects = discover_projects(&root).expect("scan projects");

        assert_eq!(projects, vec![project]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn groups_projects_by_parent_sector() {
        let root = PathBuf::from("/home/alex");
        let projects = vec![
            root.join("work/apps/jumper"),
            root.join("work/tools/runner"),
            root.join("labs/playground"),
        ];

        let sectors = group_projects(&root, projects);

        assert_eq!(sectors[0].label, "A");
        assert_eq!(sectors[0].name, "apps");
        assert_eq!(sectors[0].above, "~/work");
        assert_eq!(sectors[1].label, "B");
        assert_eq!(sectors[1].name, "labs");
        assert_eq!(sectors[1].above, "~");
        assert_eq!(sectors[2].label, "C");
        assert_eq!(sectors[2].name, "tools");
        assert_eq!(sectors[2].above, "~/work");
    }

    #[test]
    fn labels_can_grow_past_z() {
        assert_eq!(label_for_index(0), "A");
        assert_eq!(label_for_index(25), "Z");
        assert_eq!(label_for_index(26), "AA");
        assert_eq!(label_for_index(27), "AB");
        assert_eq!(label_for_index(51), "AZ");
        assert_eq!(label_for_index(52), "BA");
    }

    #[test]
    fn parses_choice_labels_and_positions() {
        assert_eq!(
            parse_choice("a1"),
            Ok(Choice {
                sector_index: 0,
                project_index: 0
            })
        );
        assert_eq!(
            parse_choice("AA 12"),
            Ok(Choice {
                sector_index: 26,
                project_index: 11
            })
        );
        assert_eq!(parse_choice(""), Err(ChoiceParseError::Empty));
        assert_eq!(parse_choice("A"), Err(ChoiceParseError::MissingNumber));
        assert_eq!(parse_choice("1"), Err(ChoiceParseError::MissingSector));
        assert_eq!(parse_choice("A0"), Err(ChoiceParseError::ZeroPosition));
    }

    #[test]
    fn parses_and_renders_project_config() {
        let contents = r#"# jumper project config
version = 1

[[projects]]
path = "/home/alex/work/jumper"
active = false

[[projects]]
path = "/tmp/with \"quote\" and # hash"
active = true # trailing comments are fine
"#;

        let config = parse_project_config(contents).expect("parse config");

        assert_eq!(
            config,
            ProjectConfig {
                version: 1,
                projects: vec![
                    ProjectConfigEntry {
                        path: PathBuf::from("/home/alex/work/jumper"),
                        active: false,
                    },
                    ProjectConfigEntry {
                        path: PathBuf::from("/tmp/with \"quote\" and # hash"),
                        active: true,
                    },
                ],
            }
        );

        let rendered = render_project_config(&config);
        let reparsed = parse_project_config(&rendered).expect("reparse rendered config");

        assert_eq!(reparsed, config);
    }

    #[test]
    fn merging_project_config_preserves_manual_active_values() {
        let existing = ProjectConfig {
            version: 1,
            projects: vec![
                ProjectConfigEntry {
                    path: PathBuf::from("/home/alex/work/jumper"),
                    active: false,
                },
                ProjectConfigEntry {
                    path: PathBuf::from("/home/alex/old/project"),
                    active: true,
                },
            ],
        };
        let discovered = vec![
            PathBuf::from("/home/alex/work/jumper"),
            PathBuf::from("/home/alex/work/new-project"),
        ];

        let merged = merge_project_config(Some(existing), discovered);

        assert_eq!(
            merged.projects,
            vec![
                ProjectConfigEntry {
                    path: PathBuf::from("/home/alex/old/project"),
                    active: true,
                },
                ProjectConfigEntry {
                    path: PathBuf::from("/home/alex/work/jumper"),
                    active: false,
                },
                ProjectConfigEntry {
                    path: PathBuf::from("/home/alex/work/new-project"),
                    active: true,
                },
            ]
        );
    }

    #[test]
    fn active_project_paths_ignores_disabled_and_missing_projects() {
        let root = temp_root("active-config");
        let active = root.join("active-project");
        let inactive = root.join("inactive-project");
        let missing = root.join("missing-project");
        fs::create_dir_all(active.join(".git")).expect("create active project");
        fs::create_dir_all(inactive.join(".git")).expect("create inactive project");

        let config = ProjectConfig {
            version: 1,
            projects: vec![
                ProjectConfigEntry {
                    path: active.clone(),
                    active: true,
                },
                ProjectConfigEntry {
                    path: inactive,
                    active: false,
                },
                ProjectConfigEntry {
                    path: missing,
                    active: true,
                },
            ],
        };

        assert_eq!(active_project_paths(&config), vec![active]);

        let _ = fs::remove_dir_all(root);
    }

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        std::env::temp_dir().join(format!("jumper-{name}-{}-{nanos}", std::process::id()))
    }
}
