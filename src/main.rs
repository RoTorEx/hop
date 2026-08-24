use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitCode, Stdio};

use hop::{
    APP_NAME, ChoiceParseError, JumpHistory, ProjectConfig, RankedProject, Sector,
    active_project_paths, cli_home_path, config_path, diff_project_config_tree, discover_projects,
    group_projects, history_path, load_jump_history, load_project_config, merge_project_config,
    parse_choice, rank_projects_by_jumps, record_jump, write_project_config,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASE_BASE_URL: &str = "https://github.com/RoTorEx/hop/releases/latest/download";
const SHELL_BINARY_ENV: &str = "HOP_SHELL_BINARY";

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const GREEN: &str = "\x1b[32m";
const BLUE: &str = "\x1b[94m";
const GRAY: &str = "\x1b[90m";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Jump,
    List,
    Help,
    Version,
    ShellInit,
    Update,
    Config,
    RecordJump,
}

#[derive(Debug)]
struct Options {
    command: Command,
    root: Option<PathBuf>,
    target: Option<String>,
    copy_path: bool,
    frequent: bool,
    color: bool,
}

fn main() -> ExitCode {
    let options = match parse_args(env::args().skip(1)) {
        Ok(options) => options,
        Err(message) => return fail(&message, colors_enabled()),
    };

    if !matches!(options.command, Command::Config | Command::RecordJump) {
        warn_if_project_config_needs_refresh(&options);
    }

    match options.command {
        Command::Help => {
            print_help(options.color);
            ExitCode::SUCCESS
        }
        Command::Version => {
            eprintln!(
                "{} {}",
                paint(options.color, BOLD, &title_case(APP_NAME)),
                paint(options.color, DIM, &format!("v{VERSION}")),
            );
            ExitCode::SUCCESS
        }
        Command::ShellInit => {
            let binary = match shell_binary_path() {
                Ok(binary) => binary,
                Err(message) => return fail(&message, options.color),
            };
            print_shell_init(&binary);
            ExitCode::SUCCESS
        }
        Command::Update => run_update(options.color),
        Command::Config => run_config(options),
        Command::RecordJump => run_record_jump(options),
        Command::Jump => run_jump(options),
        Command::List => run_list(options),
    }
}

fn run_update(color: bool) -> ExitCode {
    let release_asset = match release_asset_name() {
        Some(asset) => asset,
        None => {
            return fail(
                "hop update currently supports Linux and macOS release builds only.",
                color,
            );
        }
    };
    let current_exe = match env::current_exe() {
        Ok(path) => path,
        Err(error) => return fail(&format!("Cannot locate current executable: {error}"), color),
    };
    let update_token = match read_update_token(&current_exe) {
        Ok(token) => token,
        Err(message) => return fail(&message, color),
    };
    let release_url = format!("{RELEASE_BASE_URL}/{release_asset}");

    let temp_dir = env::temp_dir().join(format!("hop-update-{}", std::process::id()));
    if let Err(error) = recreate_dir(&temp_dir) {
        return fail(
            &format!("Cannot prepare {}: {error}", temp_dir.display()),
            color,
        );
    }

    let archive = temp_dir.join(release_asset);
    let extract_dir = temp_dir.join("extract");
    if let Err(error) = fs::create_dir_all(&extract_dir) {
        cleanup_dir(&temp_dir);
        return fail(
            &format!("Cannot prepare {}: {error}", extract_dir.display()),
            color,
        );
    }

    eprintln!(
        "{}",
        paint(color, DIM, &format!("Downloading {release_url}")),
    );
    if let Err(message) = download_release(&release_url, &archive, update_token.as_deref()) {
        cleanup_dir(&temp_dir);
        return fail(&message, color);
    }

    if !run_status(
        ProcessCommand::new("tar")
            .arg("-xzf")
            .arg(&archive)
            .arg("-C")
            .arg(&extract_dir),
    ) {
        cleanup_dir(&temp_dir);
        return fail("Could not extract release archive; install tar.", color);
    }

    let updated = extract_dir.join(APP_NAME);
    if !updated.is_file() {
        cleanup_dir(&temp_dir);
        return fail("Release archive did not contain a hop binary.", color);
    }

    if let Err(error) = set_executable(&updated) {
        cleanup_dir(&temp_dir);
        return fail(
            &format!("Cannot prepare updated executable: {error}"),
            color,
        );
    }

    let updated_shell_init = match generate_shell_init(&updated, &current_exe) {
        Ok(shell_init) => shell_init,
        Err(message) => {
            cleanup_dir(&temp_dir);
            return fail(&message, color);
        }
    };

    if let Err(error) = install_updated_binary(&updated, &current_exe) {
        cleanup_dir(&temp_dir);
        return fail(
            &format!("Cannot update {}: {error}", current_exe.display()),
            color,
        );
    }

    if let Err(error) = install_shell_init(&updated_shell_init, &current_exe) {
        cleanup_dir(&temp_dir);
        return fail(
            &format!("Updated the binary but could not refresh shell integration: {error}"),
            color,
        );
    }

    cleanup_dir(&temp_dir);
    eprintln!(
        "{}",
        paint(color, GREEN, &format!("Updated {}", current_exe.display()),),
    );
    ExitCode::SUCCESS
}

fn run_jump(options: Options) -> ExitCode {
    run_navigation(options, true)
}

fn run_list(options: Options) -> ExitCode {
    run_navigation(options, false)
}

fn run_navigation(options: Options, interactive: bool) -> ExitCode {
    if let Some(target) = options.target.as_deref() {
        match cli_home_shortcut_path(target) {
            Ok(Some(path)) => return emit_path(&path, options.copy_path, options.color),
            Ok(None) => {}
            Err(message) => return fail(&message, options.color),
        }
    }

    let (root, projects, config_source) = match projects_for_jump(&options) {
        Ok(projects) => projects,
        Err(message) => return fail(&message, options.color),
    };

    if projects.is_empty() {
        if let Some(config_source) = config_source {
            return fail(
                &format!(
                    "No active projects found in {}; edit active values or run hop config.",
                    config_source.display()
                ),
                options.color,
            );
        }

        return fail("No git projects found under the scan root.", options.color);
    }

    if options.frequent {
        let history = match load_optional_jump_history() {
            Ok(history) => history,
            Err(message) => return fail(&message, options.color),
        };
        let ranked = rank_projects_by_jumps(projects, &history);

        if let Some(target) = options.target {
            return choose_frequent_target(&ranked, &target, options.copy_path, options.color);
        }

        render_frequent(&root, &ranked, options.color);
        return if interactive {
            frequent_prompt_loop(&ranked, options.copy_path, options.color)
        } else {
            ExitCode::SUCCESS
        };
    }

    let sectors = group_projects(&root, projects);

    if let Some(target) = options.target {
        return choose_target(&sectors, &target, options.copy_path, options.color);
    }

    render(&sectors, options.color);
    if interactive {
        prompt_loop(&sectors, options.copy_path, options.color)
    } else {
        ExitCode::SUCCESS
    }
}

fn run_record_jump(options: Options) -> ExitCode {
    let Some(target) = options.target else {
        return fail("--record-jump requires a project path", options.color);
    };
    let project = PathBuf::from(target);
    if !project.join(".git").exists() {
        return ExitCode::SUCCESS;
    }
    let home = match home_dir() {
        Ok(home) => home,
        Err(message) => return fail(&message, options.color),
    };

    match record_jump(&history_path(&home), &project) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail(
            &format!("Cannot update jump history: {error}"),
            options.color,
        ),
    }
}

fn run_config(options: Options) -> ExitCode {
    let home = match home_dir() {
        Ok(home) => home,
        Err(message) => return fail(&message, options.color),
    };
    let root = options.root.unwrap_or_else(|| home.clone());
    let path = config_path(&home);

    let projects = match discover_projects(&root) {
        Ok(projects) => projects,
        Err(error) => {
            return fail(
                &format!("Cannot scan {}: {error}", root.display()),
                options.color,
            );
        }
    };

    let existing = match load_optional_project_config(&path) {
        Ok(existing) => existing,
        Err(message) => return fail(&message, options.color),
    };
    let config = merge_project_config(existing, root.clone(), projects);
    let total = config.projects.len();
    let active = config
        .projects
        .iter()
        .filter(|project| project.active)
        .count();

    if let Err(error) = write_project_config(&path, &config) {
        return fail(
            &format!("Cannot write {}: {error}", path.display()),
            options.color,
        );
    }

    eprintln!(
        "{}",
        paint(
            options.color,
            GREEN,
            &format!("Updated {} ({active}/{total} active)", path.display()),
        ),
    );
    eprintln!(
        "{}",
        paint(
            options.color,
            DIM,
            "Edit active = false to hide projects from hop.",
        ),
    );
    ExitCode::SUCCESS
}

fn projects_for_jump(
    options: &Options,
) -> Result<(PathBuf, Vec<PathBuf>, Option<PathBuf>), String> {
    if let Some(root) = &options.root {
        let projects = discover_projects(root)
            .map_err(|error| format!("Cannot scan {}: {error}", root.display()))?;
        return Ok((root.clone(), projects, None));
    }

    let home = home_dir()?;
    let path = config_path(&home);
    if path.exists() {
        let config = load_project_config(&path)
            .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
        return Ok((home, active_project_paths(&config), Some(path)));
    }

    Err(format!(
        "Project config not found at {}; run `hop config` first.",
        path.display()
    ))
}

fn warn_if_project_config_needs_refresh(options: &Options) {
    let home = match home_dir() {
        Ok(home) => home,
        Err(_) => return,
    };
    let path = config_path(&home);
    let config = match load_project_config(&path) {
        Ok(config) => config,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if !uses_default_project_config(options) {
                config_warning(
                    &format!(
                        "Project config not found at {}; run `hop config`.",
                        path.display()
                    ),
                    options.color,
                );
            }
            return;
        }
        Err(error) => {
            if !uses_default_project_config(options) {
                config_warning(
                    &format!(
                        "Cannot check project config freshness: cannot read {}: {error}.",
                        path.display()
                    ),
                    options.color,
                );
            }
            return;
        }
    };

    let scan_root = config.scan_root.as_deref().unwrap_or(&home);
    let discovered = match discover_projects(scan_root) {
        Ok(projects) => projects,
        Err(error) => {
            config_warning(
                &format!(
                    "Cannot check project tree against config: cannot scan {}: {error}. Run {} to refresh it.",
                    scan_root.display(),
                    refresh_command(scan_root, &home),
                ),
                options.color,
            );
            return;
        }
    };

    let diff = diff_project_config_tree(&config, &discovered);
    if diff.is_empty() {
        return;
    }

    config_warning(
        &format!(
            "Project tree differs from config: {}. Run {} to refresh it.",
            project_tree_diff_summary(&diff),
            refresh_command(scan_root, &home),
        ),
        options.color,
    );
}

fn uses_default_project_config(options: &Options) -> bool {
    matches!(options.command, Command::Jump | Command::List) && options.root.is_none()
}

fn project_tree_diff_summary(diff: &hop::ProjectTreeDiff) -> String {
    let mut parts = Vec::new();
    let unconfigured = diff.unconfigured_projects.len();
    let stale = diff.stale_projects.len();

    if unconfigured > 0 {
        parts.push(format!("{unconfigured} new {}", project_word(unconfigured)));
    }
    if stale > 0 {
        parts.push(format!(
            "{stale} missing configured {}",
            project_word(stale)
        ));
    }

    match parts.as_slice() {
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => parts.join(", "),
    }
}

fn project_word(count: usize) -> &'static str {
    if count == 1 { "project" } else { "projects" }
}

fn refresh_command(scan_root: &Path, home: &Path) -> String {
    if scan_root == home {
        "`hop config`".to_owned()
    } else {
        format!(
            "`hop config --root {}`",
            shell_quote(&scan_root.display().to_string())
        )
    }
}

fn load_optional_project_config(path: &Path) -> Result<Option<ProjectConfig>, String> {
    match load_project_config(path) {
        Ok(config) => Ok(Some(config)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Cannot read {}: {error}", path.display())),
    }
}

fn load_optional_jump_history() -> Result<JumpHistory, String> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Ok(JumpHistory::default());
    };
    let path = history_path(&home);
    match load_jump_history(&path) {
        Ok(history) => Ok(history),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(JumpHistory::default()),
        Err(error) => Err(format!("Cannot read {}: {error}", path.display())),
    }
}

fn home_dir() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set; pass --root <dir>".to_owned())
}

fn cli_home_shortcut_path(target: &str) -> Result<Option<PathBuf>, String> {
    if target == "~" {
        return home_dir().map(|home| Some(cli_home_path(&home)));
    }

    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Ok(None);
    };

    Ok(cli_home_shortcut_path_for_home(target, &home))
}

fn cli_home_shortcut_path_for_home(target: &str, home: &Path) -> Option<PathBuf> {
    if target == "~" || Path::new(target) == home {
        Some(cli_home_path(home))
    } else {
        None
    }
}

fn prompt_loop(sectors: &[Sector], copy_path: bool, color: bool) -> ExitCode {
    loop {
        eprint!(
            "  {} {} {}: ",
            paint(color, BLUE, ">"),
            paint(color, BOLD, if copy_path { "copy" } else { "jump to" }),
            paint(color, DIM, "<sector><position>"),
        );
        if io::stderr().flush().is_err() {
            return ExitCode::from(1);
        }

        let input = match read_choice() {
            Ok(input) => input,
            Err(_) => return ExitCode::from(1),
        };

        let choice = match parse_choice(&input) {
            Ok(choice) => choice,
            Err(ChoiceParseError::Empty) => return ExitCode::from(1),
            Err(error) => {
                warn(error.message(), color);
                continue;
            }
        };

        match path_for_choice(sectors, choice) {
            Ok(path) => return emit_path(path, copy_path, color),
            Err(message) => warn(message, color),
        }
    }
}

fn frequent_prompt_loop(projects: &[RankedProject], copy_path: bool, color: bool) -> ExitCode {
    loop {
        eprint!(
            "  {} {} {}: ",
            paint(color, BLUE, ">"),
            paint(color, BOLD, if copy_path { "copy" } else { "jump to" }),
            paint(color, DIM, "<position>"),
        );
        if io::stderr().flush().is_err() {
            return ExitCode::from(1);
        }

        let input = match read_choice() {
            Ok(input) => input,
            Err(_) => return ExitCode::from(1),
        };
        if input.is_empty() {
            return ExitCode::from(1);
        }

        match frequent_path_for_target(projects, &input) {
            Ok(path) => return emit_path(path, copy_path, color),
            Err(message) => warn(message, color),
        }
    }
}

fn choose_frequent_target(
    projects: &[RankedProject],
    target: &str,
    copy_path: bool,
    color: bool,
) -> ExitCode {
    match frequent_path_for_target(projects, target) {
        Ok(path) => emit_path(path, copy_path, color),
        Err(message) => fail(message, color),
    }
}

fn frequent_path_for_target<'a>(
    projects: &'a [RankedProject],
    target: &str,
) -> Result<&'a Path, &'static str> {
    let target = target.trim();
    if target.is_empty() {
        return Err("empty input cancels");
    }
    if !target.chars().all(|ch| ch.is_ascii_digit()) {
        return Err("expected a positive project position, for example 1");
    }
    let position = target
        .parse::<usize>()
        .map_err(|_| "expected a positive project position, for example 1")?;
    if position == 0 {
        return Err("project position starts at 1");
    }

    projects
        .get(position - 1)
        .map(|project| project.path.as_path())
        .ok_or("No such project")
}

fn choose_target(sectors: &[Sector], target: &str, copy_path: bool, color: bool) -> ExitCode {
    let choice = match parse_choice(target) {
        Ok(choice) => choice,
        Err(error) => return fail(error.message(), color),
    };

    match path_for_choice(sectors, choice) {
        Ok(path) => emit_path(path, copy_path, color),
        Err(message) => fail(message, color),
    }
}

fn path_for_choice(
    sectors: &[Sector],
    choice: hop::Choice,
) -> Result<&std::path::Path, &'static str> {
    let Some(sector) = sectors.get(choice.sector_index) else {
        return Err("No such sector");
    };
    let Some(path) = sector.paths.get(choice.project_index) else {
        return Err("No such project");
    };

    Ok(path)
}

fn emit_path(path: &std::path::Path, copy_path: bool, color: bool) -> ExitCode {
    if copy_path {
        return copy_to_clipboard(path, color);
    }

    if io::stdout().is_terminal() {
        eprintln!(
            "{}",
            paint(
                color,
                YELLOW,
                "Shell integration is not active, so the hop executable cannot change this shell's directory.",
            ),
        );
        eprintln!(
            "Selected: {}\nActivate it with: {}",
            path.display(),
            shell_activation_command(),
        );
        return ExitCode::from(1);
    }

    println!("{}", path.display());
    ExitCode::SUCCESS
}

fn copy_to_clipboard(path: &std::path::Path, color: bool) -> ExitCode {
    let value = path.display().to_string();
    let commands: &[(&str, &[&str])] = &[
        ("pbcopy", &[]),
        ("wl-copy", &[]),
        ("xclip", &["-selection", "clipboard"]),
        ("xsel", &["--clipboard", "--input"]),
    ];

    for (program, args) in commands {
        let Ok(mut child) = ProcessCommand::new(program)
            .args(*args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            continue;
        };

        let Some(mut stdin) = child.stdin.take() else {
            continue;
        };
        if stdin.write_all(value.as_bytes()).is_err() {
            continue;
        }
        drop(stdin);

        match child.wait() {
            Ok(status) if status.success() => {
                eprintln!(
                    "  {} {}",
                    paint(color, GREEN, "copied"),
                    paint(color, DIM, &value),
                );
                return ExitCode::SUCCESS;
            }
            _ => continue,
        }
    }

    fail(
        "Could not copy path; install pbcopy, wl-copy, xclip, or xsel.",
        color,
    )
}

fn release_asset_name() -> Option<&'static str> {
    release_asset_name_for(env::consts::OS, env::consts::ARCH)
}

fn release_asset_name_for(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("linux", "x86_64") => Some("hop-linux-x86_64.tar.gz"),
        ("linux", "aarch64") => Some("hop-linux-aarch64.tar.gz"),
        ("macos", "x86_64") => Some("hop-macos-x86_64.tar.gz"),
        ("macos", "aarch64") => Some("hop-macos-aarch64.tar.gz"),
        _ => None,
    }
}

fn read_update_token(current_exe: &Path) -> Result<Option<String>, String> {
    let Some(install_home) = installation_home(current_exe) else {
        return Ok(None);
    };
    let token_file = install_home.join("gh-token");
    let token = match fs::read_to_string(&token_file) {
        Ok(token) => token,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            return Err(
                "Cannot read stored GitHub token; rerun the installer with GH_INSTALLER_TOKEN."
                    .to_owned(),
            );
        }
    };
    let token = token.trim();
    if token.is_empty() {
        return Ok(None);
    }
    if token.contains(['\r', '\n']) {
        return Err(
            "Stored GitHub token is invalid; rerun the installer with GH_INSTALLER_TOKEN."
                .to_owned(),
        );
    }

    Ok(Some(token.to_owned()))
}

fn download_release(
    url: &str,
    destination: &Path,
    update_token: Option<&str>,
) -> Result<(), String> {
    if let Some(token) = update_token {
        if !command_exists("curl") {
            return Err("Authenticated updates require curl; install curl and retry.".to_owned());
        }
        if run_curl_config(&authenticated_curl_config(url, destination, token)) {
            return Ok(());
        }
        return Err(
            "Could not download latest release with stored GitHub token; rerun the installer with GH_INSTALLER_TOKEN or check token access."
                .to_owned(),
        );
    }

    if run_status(
        ProcessCommand::new("curl")
            .arg("-fsSL")
            .arg(url)
            .arg("-o")
            .arg(destination),
    ) {
        return Ok(());
    }

    if run_status(
        ProcessCommand::new("wget")
            .arg("-qO")
            .arg(destination)
            .arg(url),
    ) {
        return Ok(());
    }

    Err("Could not download latest release; install curl or wget.".to_owned())
}

fn authenticated_curl_config(url: &str, destination: &Path, token: &str) -> String {
    format!(
        "fail\nshow-error\nsilent\nlocation\nurl = \"{}\"\noutput = \"{}\"\nheader = \"Authorization: Bearer {}\"\n",
        curl_config_quote(url),
        curl_config_quote(&destination.display().to_string()),
        curl_config_quote(token),
    )
}

fn curl_config_quote(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn run_curl_config(config: &str) -> bool {
    let Ok(mut child) = ProcessCommand::new("curl")
        .arg("-K")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let Some(mut stdin) = child.stdin.take() else {
        return false;
    };
    if stdin.write_all(config.as_bytes()).is_err() {
        return false;
    }
    drop(stdin);

    match child.wait() {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

fn command_exists(program: &str) -> bool {
    ProcessCommand::new(program)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn install_updated_binary(source: &Path, destination: &Path) -> io::Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::other("executable has no parent directory"))?;
    let temp_destination = parent.join(format!(".hop-update-{}", std::process::id()));

    let result = (|| {
        fs::copy(source, &temp_destination)?;
        set_executable(&temp_destination)?;
        fs::rename(&temp_destination, destination)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_destination);
    }

    result
}

fn generate_shell_init(binary: &Path, installed_binary: &Path) -> Result<Vec<u8>, String> {
    let output = ProcessCommand::new(binary)
        .arg("--shell-init")
        .env(SHELL_BINARY_ENV, installed_binary)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("Cannot generate updated shell integration: {error}"))?;

    if !output.status.success() || output.stdout.is_empty() {
        return Err("Updated binary could not generate shell integration.".to_owned());
    }

    Ok(output.stdout)
}

fn install_shell_init(contents: &[u8], installed_binary: &Path) -> io::Result<()> {
    let install_home = installation_home(installed_binary)
        .ok_or_else(|| io::Error::other("executable has no parent directory"))?;
    let destination = install_home.join("init.zsh");
    let temporary = install_home.join(format!(".hop-init-update-{}", std::process::id()));

    let result = (|| {
        fs::write(&temporary, contents)?;
        set_shell_init_permissions(&temporary)?;
        fs::rename(&temporary, destination)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }

    result
}

fn recreate_dir(path: &Path) -> io::Result<()> {
    cleanup_dir(path);
    fs::create_dir_all(path)
}

fn cleanup_dir(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

fn run_status(command: &mut ProcessCommand) -> bool {
    match command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(status) => status.success(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(_) => false,
    }
}

#[cfg(unix)]
fn set_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
}

#[cfg(unix)]
fn set_shell_init_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o644))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn set_shell_init_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut command = Command::Jump;
    let mut root = None;
    let mut target = None;
    let mut copy_path = false;
    let mut frequent = false;
    let mut color = colors_enabled();
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => command = Command::Help,
            "-v" | "-V" | "--version" => command = Command::Version,
            "--shell-init" => command = Command::ShellInit,
            "--copy-path" => copy_path = true,
            "--frequent" => frequent = true,
            "--record-jump" => {
                if command != Command::Jump || target.is_some() {
                    return Err(format!("unexpected argument: {arg}"));
                }
                let Some(value) = args.next() else {
                    return Err("--record-jump requires a project path".to_owned());
                };
                command = Command::RecordJump;
                target = Some(value);
            }
            "--no-color" => color = false,
            "--root" => {
                let Some(value) = args.next() else {
                    return Err("--root requires a directory".to_owned());
                };
                root = Some(PathBuf::from(value));
            }
            _ if arg.starts_with("--root=") => {
                let value = arg
                    .split_once('=')
                    .map(|(_, value)| value)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "--root requires a directory".to_owned())?;
                root = Some(PathBuf::from(value));
            }
            _ if arg.starts_with('-') => return Err(format!("unknown argument: {arg}")),
            "config" if target.is_none() => {
                if command != Command::Jump {
                    return Err(format!("unexpected argument: {arg}"));
                }
                command = Command::Config;
            }
            "list" if target.is_none() => {
                if command != Command::Jump {
                    return Err(format!("unexpected argument: {arg}"));
                }
                command = Command::List;
            }
            "update" if target.is_none() => {
                if command != Command::Jump {
                    return Err(format!("unexpected argument: {arg}"));
                }
                command = Command::Update;
            }
            _ => {
                if matches!(
                    command,
                    Command::List | Command::Update | Command::Config | Command::RecordJump
                ) {
                    return Err(format!("unexpected argument: {arg}"));
                }
                if target.is_some() {
                    return Err(format!("unexpected argument: {arg}"));
                }
                target = Some(arg);
            }
        }
    }

    Ok(Options {
        command,
        root,
        target,
        copy_path,
        frequent,
        color,
    })
}

fn render(sectors: &[Sector], color: bool) {
    eprintln!();
    eprintln!(
        "  {} {} {}",
        paint(color, YELLOW, "*"),
        paint(color, BOLD, &title_case(APP_NAME)),
        paint(color, DIM, &format!("(v{VERSION})")),
    );
    eprintln!();
    eprintln!(
        "  {} {} {}",
        paint(color, DIM, "--------------"),
        paint(color, BLUE, "Projects"),
        paint(color, DIM, "--------------"),
    );

    for sector in sectors {
        eprintln!();
        eprintln!(
            "  {}.  {} {}",
            paint(color, YELLOW, &sector.label),
            paint(color, BOLD, &sector.name),
            paint(color, GRAY, &format!("({})", sector.above)),
        );

        for (index, path) in sector.paths.iter().enumerate() {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("?");
            eprintln!(
                "     {} {}",
                paint(color, CYAN, &format!("{:>2})", index + 1)),
                paint(color, GREEN, name),
            );
        }
    }

    eprintln!();
}

fn render_frequent(scan_root: &Path, projects: &[RankedProject], color: bool) {
    eprintln!();
    eprintln!(
        "  {} {} {}",
        paint(color, YELLOW, "*"),
        paint(color, BOLD, &title_case(APP_NAME)),
        paint(color, DIM, &format!("(v{VERSION})")),
    );
    eprintln!();
    eprintln!(
        "  {} {} {}",
        paint(color, DIM, "-----------"),
        paint(color, BLUE, "Frequent projects"),
        paint(color, DIM, "-----------"),
    );
    eprintln!();

    let position_width = projects.len().to_string().len().max(1);
    let jump_width = projects
        .iter()
        .map(|project| project.jumps.to_string().len())
        .max()
        .unwrap_or(1);

    for (index, project) in projects.iter().enumerate() {
        let name = project
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("?");
        let location = project_parent_display(scan_root, &project.path);
        eprintln!(
            "  {} {} {} {}",
            paint(color, CYAN, &format!("{:>position_width$})", index + 1)),
            paint(color, GREEN, name),
            paint(color, GRAY, &format!("({location})")),
            paint(
                color,
                DIM,
                &format!(
                    "{:>jump_width$} {}",
                    project.jumps,
                    if project.jumps == 1 { "jump" } else { "jumps" }
                ),
            ),
        );
    }

    eprintln!();
}

fn project_parent_display(scan_root: &Path, project: &Path) -> String {
    let parent = project.parent().unwrap_or_else(|| Path::new("/"));
    let display = match parent.strip_prefix(scan_root) {
        Ok(relative) if relative.as_os_str().is_empty() => "~".to_owned(),
        Ok(relative) => format!("~/{}", relative.display()),
        Err(_) => parent.display().to_string(),
    };

    if display.ends_with('/') {
        display
    } else {
        format!("{display}/")
    }
}

fn read_choice() -> io::Result<String> {
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_owned())
}

fn fail(message: &str, color: bool) -> ExitCode {
    eprintln!("{}", paint(color, RED, message));
    ExitCode::from(1)
}

fn warn(message: &str, color: bool) {
    eprintln!(
        "  {} {}",
        paint(color, RED, "x"),
        paint(color, RED, message)
    );
}

fn config_warning(message: &str, color: bool) {
    eprintln!(
        "  {} {}",
        paint(color, YELLOW, "!"),
        paint(color, YELLOW, message),
    );
}

fn paint(color: bool, code: &str, value: &str) -> String {
    if color {
        format!("{code}{value}{RESET}")
    } else {
        value.to_owned()
    }
}

fn colors_enabled() -> bool {
    env::var_os("NO_COLOR").is_none()
        && env::var("TERM").map_or(true, |terminal| terminal != "dumb")
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => value.to_owned(),
    }
}

fn print_help(color: bool) {
    eprintln!(
        "{} {}

Tiny interactive project navigator for shells on local machines, VMs, and VPS hosts.

{}:
    hop [<target>] [--copy-path] [--frequent] [--root <dir>] [--no-color]
    hop list [--frequent] [--root <dir>] [--no-color]
    hop ~
    hop config [--root <dir>]
    hop update
    hop --version

{}:
    list             Print the project list without prompting for a selection
    config           Create or update the project config
    update           Update this executable from the latest GitHub release

{}:
    --copy-path      Copy the selected path instead of printing it
    --frequent       Rank all projects by successful jump count
    --root <dir>      Scan a directory instead of $HOME
    --no-color        Disable ANSI color output
    -v, -V, --version Print version
    -h, --help        Print help

The project UI writes to stderr. List mode prints it without prompting for a
selection. Jump mode prints only the selected project path to stdout, so a
shell wrapper can safely cd into it. Target ~ prints the hop home directory.
Copy mode writes no stdout and copies the selected project path to the
clipboard. Frequent mode uses numeric positions instead of sector labels.",
        paint(color, BOLD, &title_case(APP_NAME)),
        paint(color, DIM, &format!("v{VERSION}")),
        paint(color, BLUE, "USAGE"),
        paint(color, BLUE, "COMMANDS"),
        paint(color, BLUE, "OPTIONS"),
    );
}

fn shell_binary_path() -> Result<PathBuf, String> {
    if let Some(binary) = env::var_os(SHELL_BINARY_ENV).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(binary));
    }

    env::current_exe().map_err(|error| format!("Cannot locate current executable: {error}"))
}

fn print_shell_init(binary: &Path) {
    println!("{}", shell_init(binary));
}

fn shell_init(binary: &Path) -> String {
    let binary_dir = binary.parent().unwrap_or_else(|| Path::new("."));
    let binary = shell_quote(&binary.display().to_string());
    let binary_dir = shell_quote(&binary_dir.display().to_string());

    format!(
        r#"# x-cli-hop shell bridge
_hop_bin_dir={binary_dir}
case ":$PATH:" in
    *":$_hop_bin_dir:"*) ;;
    *) export PATH="$_hop_bin_dir:$PATH" ;;
esac
unset _hop_bin_dir

unalias j 2>/dev/null || true
unalias hop 2>/dev/null || true
if [ -n "${{ZSH_VERSION:-}}" ]; then
    unfunction j 2>/dev/null || true
    unfunction hop 2>/dev/null || true
else
    unset -f j 2>/dev/null || true
    unset -f hop 2>/dev/null || true
fi

function hop {{
    local destination
    local exit_status
    destination="$(command {binary} "$@")"
    exit_status=$?
    if [ "$exit_status" -ne 0 ]; then
        return "$exit_status"
    fi
    if [ -z "$destination" ]; then
        return 0
    fi
    if [ ! -d "$destination" ]; then
        printf '%s\n' "hop: invalid destination: $destination" >&2
        return 1
    fi
    builtin cd -- "$destination" || return $?
    command {binary} --record-jump "$destination" --no-color || true
}}"#,
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn installation_home(binary: &Path) -> Option<&Path> {
    let parent = binary.parent()?;
    if parent.file_name() == Some(OsStr::new("bin")) {
        parent.parent()
    } else {
        Some(parent)
    }
}

fn shell_activation_command() -> String {
    env::current_exe()
        .ok()
        .and_then(|binary| installation_home(&binary).map(|home| home.join("init.zsh")))
        .map_or_else(
            || ". \"$HOME/.x-cli-hop/init.zsh\"".to_owned(),
            |init| format!(". {}", shell_quote(&init.display().to_string())),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn cli_home_shortcut_accepts_literal_and_shell_expanded_home() {
        let home = PathBuf::from("/home/alex");

        assert_eq!(
            cli_home_shortcut_path_for_home("~", &home),
            Some(PathBuf::from("/home/alex/.x-cli-hop"))
        );
        assert_eq!(
            cli_home_shortcut_path_for_home("/home/alex", &home),
            Some(PathBuf::from("/home/alex/.x-cli-hop"))
        );
        assert_eq!(cli_home_shortcut_path_for_home("A1", &home), None);
    }

    #[test]
    fn shell_init_removes_legacy_j_and_wraps_only_hop() {
        let init = shell_init(Path::new("/opt/hop home/bin/hop"));

        assert!(init.contains("_hop_bin_dir='/opt/hop home/bin'"));
        assert!(init.contains("unalias j"));
        assert!(init.contains("unalias hop"));
        assert!(init.contains("unfunction j"));
        assert!(init.contains("unfunction hop"));
        assert!(init.contains("unset -f j"));
        assert!(init.contains("unset -f hop"));
        assert!(init.contains("function hop"));
        assert!(!init.contains("\nfunction j {"));
        assert!(!init.contains("_hop_dispatch"));
        assert!(!init.contains("for arg in"));
        assert!(init.contains("command '/opt/hop home/bin/hop' \"$@\""));
        assert!(init.contains("builtin cd -- \"$destination\""));
        assert!(init.contains(
            "command '/opt/hop home/bin/hop' --record-jump \"$destination\" --no-color || true"
        ));
        assert!(init.contains("return \"$exit_status\""));
        assert!(init.contains("invalid destination"));
    }

    #[test]
    fn shell_quote_handles_apostrophes() {
        assert_eq!(shell_quote("/tmp/alex's/hop"), "'/tmp/alex'\"'\"'s/hop'");
    }

    #[test]
    fn installation_home_supports_legacy_and_bin_layouts() {
        assert_eq!(
            installation_home(Path::new("/home/alex/.x-cli-hop/hop")),
            Some(Path::new("/home/alex/.x-cli-hop")),
        );
        assert_eq!(
            installation_home(Path::new("/home/alex/.x-cli-hop/bin/hop")),
            Some(Path::new("/home/alex/.x-cli-hop")),
        );
    }

    #[test]
    fn summarizes_project_tree_diffs_for_terminal_warning() {
        let diff = hop::ProjectTreeDiff {
            unconfigured_projects: vec![
                PathBuf::from("/home/alex/work/new-1"),
                PathBuf::from("/home/alex/work/new-2"),
            ],
            stale_projects: vec![PathBuf::from("/home/alex/work/old")],
        };

        assert_eq!(
            project_tree_diff_summary(&diff),
            "2 new projects and 1 missing configured project"
        );
        assert_eq!(
            refresh_command(Path::new("/srv"), Path::new("/home/alex")),
            "`hop config --root '/srv'`"
        );
        assert_eq!(
            refresh_command(Path::new("/home/alex"), Path::new("/home/alex")),
            "`hop config`"
        );
    }

    #[test]
    fn installs_shell_init_at_installation_home() {
        let root = temp_root("shell-init");
        let binary = root.join("bin/hop");
        fs::create_dir_all(binary.parent().expect("binary parent")).expect("create bin dir");

        install_shell_init(b"bridge\n", &binary).expect("install shell init");

        let init = root.join("init.zsh");
        assert_eq!(fs::read(&init).expect("read shell init"), b"bridge\n");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            assert_eq!(
                fs::metadata(&init)
                    .expect("shell init metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o644,
            );
        }

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reads_update_token_from_installation_home() {
        let root = temp_root("update-token");
        let binary = root.join("bin/hop");
        fs::create_dir_all(binary.parent().expect("binary parent")).expect("create bin dir");
        fs::write(root.join("gh-token"), "test-token\n").expect("write token");

        assert_eq!(
            read_update_token(&binary).expect("read update token"),
            Some("test-token".to_owned()),
        );

        let _ = fs::remove_dir_all(root);
    }

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        env::temp_dir().join(format!("hop-main-{name}-{}-{nanos}", std::process::id()))
    }

    #[test]
    fn parse_args_accepts_lowercase_direct_target() {
        let options = parse_args(["b1".to_owned()].into_iter()).expect("parse direct target");

        assert_eq!(options.command, Command::Jump);
        assert_eq!(options.target.as_deref(), Some("b1"));
    }

    #[test]
    fn parse_args_accepts_frequent_view_and_numeric_target() {
        let options = parse_args(["--frequent".to_owned(), "2".to_owned()].into_iter())
            .expect("parse frequent target");

        assert_eq!(options.command, Command::Jump);
        assert!(options.frequent);
        assert_eq!(options.target.as_deref(), Some("2"));
    }

    #[test]
    fn parse_args_accepts_non_interactive_list_views() {
        let options = parse_args(["list".to_owned()].into_iter()).expect("parse list");

        assert_eq!(options.command, Command::List);
        assert!(!options.frequent);
        assert_eq!(options.target, None);

        let options = parse_args(["list".to_owned(), "--frequent".to_owned()].into_iter())
            .expect("parse frequent list");

        assert_eq!(options.command, Command::List);
        assert!(options.frequent);
        assert_eq!(options.target, None);
    }

    #[test]
    fn parse_args_rejects_a_target_for_list() {
        let error = parse_args(["list".to_owned(), "A1".to_owned()].into_iter())
            .expect_err("list target should fail");

        assert_eq!(error, "unexpected argument: A1");
    }

    #[test]
    fn parse_args_accepts_internal_jump_recording() {
        let options = parse_args(
            [
                "--record-jump".to_owned(),
                "/work/hop".to_owned(),
                "--no-color".to_owned(),
            ]
            .into_iter(),
        )
        .expect("parse jump recording");

        assert_eq!(options.command, Command::RecordJump);
        assert_eq!(options.target.as_deref(), Some("/work/hop"));
        assert!(!options.color);
    }

    #[test]
    fn frequent_targets_are_numeric_positions() {
        let projects = vec![
            RankedProject {
                path: PathBuf::from("/work/favorite"),
                jumps: 8,
            },
            RankedProject {
                path: PathBuf::from("/work/other"),
                jumps: 2,
            },
        ];

        assert_eq!(
            frequent_path_for_target(&projects, "2"),
            Ok(Path::new("/work/other"))
        );
        assert_eq!(
            frequent_path_for_target(&projects, "A1"),
            Err("expected a positive project position, for example 1")
        );
        assert_eq!(
            frequent_path_for_target(&projects, "0"),
            Err("project position starts at 1")
        );
    }

    #[test]
    fn release_asset_names_cover_supported_platforms() {
        assert_eq!(
            release_asset_name_for("linux", "x86_64"),
            Some("hop-linux-x86_64.tar.gz")
        );
        assert_eq!(
            release_asset_name_for("linux", "aarch64"),
            Some("hop-linux-aarch64.tar.gz")
        );
        assert_eq!(
            release_asset_name_for("macos", "x86_64"),
            Some("hop-macos-x86_64.tar.gz")
        );
        assert_eq!(
            release_asset_name_for("macos", "aarch64"),
            Some("hop-macos-aarch64.tar.gz")
        );
        assert_eq!(release_asset_name_for("windows", "x86_64"), None);
    }
}
