use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const APP_NAME: &str = "jumper";

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

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        std::env::temp_dir().join(format!("jumper-{name}-{}-{nanos}", std::process::id()))
    }
}
