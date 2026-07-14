use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitCode, Stdio};

use jumper::{
    APP_NAME, ChoiceParseError, ProjectConfig, Sector, active_project_paths, cli_home_path,
    config_path, discover_projects, group_projects, load_project_config, merge_project_config,
    parse_choice, write_project_config,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASE_BASE_URL: &str = "https://github.com/RoTorEx/jumper/releases/latest/download";
const SHELL_BINARY_ENV: &str = "JUMPER_SHELL_BINARY";

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
    Help,
    Version,
    ShellInit,
    Update,
    Config,
}

#[derive(Debug)]
struct Options {
    command: Command,
    root: Option<PathBuf>,
    target: Option<String>,
    copy_path: bool,
    color: bool,
}

fn main() -> ExitCode {
    let options = match parse_args(env::args().skip(1)) {
        Ok(options) => options,
        Err(message) => return fail(&message, colors_enabled()),
    };

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
        Command::Jump => run_jump(options),
    }
}

fn run_update(color: bool) -> ExitCode {
    let release_asset = match release_asset_name() {
        Some(asset) => asset,
        None => {
            return fail(
                "jumper update currently supports Linux and macOS release builds only.",
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

    let temp_dir = env::temp_dir().join(format!("jumper-update-{}", std::process::id()));
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
        return fail("Release archive did not contain a jumper binary.", color);
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
                    "No active projects found in {}; edit active values or run jumper config.",
                    config_source.display()
                ),
                options.color,
            );
        }

        return fail("No git projects found under the scan root.", options.color);
    }

    let sectors = group_projects(&root, projects);

    if let Some(target) = options.target {
        return choose_target(&sectors, &target, options.copy_path, options.color);
    }

    render(&sectors, options.color);
    prompt_loop(&sectors, options.copy_path, options.color)
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
    let config = merge_project_config(existing, projects);
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
            "Edit active = false to hide projects from jumper.",
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
        "Project config not found at {}; run `jumper config` first.",
        path.display()
    ))
}

fn load_optional_project_config(path: &Path) -> Result<Option<ProjectConfig>, String> {
    match load_project_config(path) {
        Ok(config) => Ok(Some(config)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
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
    choice: jumper::Choice,
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
                "Shell integration is not active, so the jumper executable cannot change this shell's directory.",
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
        ("linux", "x86_64") => Some("jumper-linux-x86_64.tar.gz"),
        ("linux", "aarch64") => Some("jumper-linux-aarch64.tar.gz"),
        ("macos", "x86_64") => Some("jumper-macos-x86_64.tar.gz"),
        ("macos", "aarch64") => Some("jumper-macos-aarch64.tar.gz"),
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
    let temp_destination = parent.join(format!(".jumper-update-{}", std::process::id()));

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
    let temporary = install_home.join(format!(".jumper-init-update-{}", std::process::id()));

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
    let mut color = colors_enabled();
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => command = Command::Help,
            "-v" | "-V" | "--version" => command = Command::Version,
            "--shell-init" => command = Command::ShellInit,
            "--copy-path" => copy_path = true,
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
            "update" if target.is_none() => {
                if command != Command::Jump {
                    return Err(format!("unexpected argument: {arg}"));
                }
                command = Command::Update;
            }
            _ => {
                if matches!(command, Command::Update | Command::Config) {
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
    jumper [<target>] [--copy-path] [--root <dir>] [--no-color]
    jumper ~
    jumper config [--root <dir>]
    jumper update
    jumper --version

{}:
    config           Create or update the project config
    update           Update this executable from the latest GitHub release

{}:
    --copy-path      Copy the selected path instead of printing it
    --root <dir>      Scan a directory instead of $HOME
    --no-color        Disable ANSI color output
    -v, -V, --version Print version
    -h, --help        Print help

The interactive UI writes to stderr. Jump mode prints only the selected project
path to stdout, so a shell wrapper can safely cd into it. Target ~ prints the
jumper home directory. Copy mode writes no stdout and copies the selected
project path to the clipboard.",
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
        r#"# x-cli-jumper shell bridge
_jumper_bin_dir={binary_dir}
case ":$PATH:" in
    *":$_jumper_bin_dir:"*) ;;
    *) export PATH="$_jumper_bin_dir:$PATH" ;;
esac
unset _jumper_bin_dir

unalias j 2>/dev/null || true
unalias jumper 2>/dev/null || true
if [ -n "${{ZSH_VERSION:-}}" ]; then
    unfunction j 2>/dev/null || true
    unfunction jumper 2>/dev/null || true
else
    unset -f j 2>/dev/null || true
    unset -f jumper 2>/dev/null || true
fi

function jumper {{
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
        printf '%s\n' "jumper: invalid destination: $destination" >&2
        return 1
    fi
    builtin cd -- "$destination"
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
            || ". \"$HOME/.x-cli-jumper/init.zsh\"".to_owned(),
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
            Some(PathBuf::from("/home/alex/.x-cli-jumper"))
        );
        assert_eq!(
            cli_home_shortcut_path_for_home("/home/alex", &home),
            Some(PathBuf::from("/home/alex/.x-cli-jumper"))
        );
        assert_eq!(cli_home_shortcut_path_for_home("A1", &home), None);
    }

    #[test]
    fn shell_init_removes_legacy_j_and_wraps_only_jumper() {
        let init = shell_init(Path::new("/opt/jumper home/bin/jumper"));

        assert!(init.contains("_jumper_bin_dir='/opt/jumper home/bin'"));
        assert!(init.contains("unalias j"));
        assert!(init.contains("unalias jumper"));
        assert!(init.contains("unfunction j"));
        assert!(init.contains("unfunction jumper"));
        assert!(init.contains("unset -f j"));
        assert!(init.contains("unset -f jumper"));
        assert!(init.contains("function jumper"));
        assert!(!init.contains("\nfunction j {"));
        assert!(!init.contains("_jumper_dispatch"));
        assert!(!init.contains("for arg in"));
        assert!(init.contains("command '/opt/jumper home/bin/jumper' \"$@\""));
        assert!(init.contains("builtin cd -- \"$destination\""));
        assert!(init.contains("return \"$exit_status\""));
        assert!(init.contains("invalid destination"));
    }

    #[test]
    fn shell_quote_handles_apostrophes() {
        assert_eq!(
            shell_quote("/tmp/alex's/jumper"),
            "'/tmp/alex'\"'\"'s/jumper'"
        );
    }

    #[test]
    fn installation_home_supports_legacy_and_bin_layouts() {
        assert_eq!(
            installation_home(Path::new("/home/alex/.x-cli-jumper/jumper")),
            Some(Path::new("/home/alex/.x-cli-jumper")),
        );
        assert_eq!(
            installation_home(Path::new("/home/alex/.x-cli-jumper/bin/jumper")),
            Some(Path::new("/home/alex/.x-cli-jumper")),
        );
    }

    #[test]
    fn installs_shell_init_at_installation_home() {
        let root = temp_root("shell-init");
        let binary = root.join("bin/jumper");
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
        let binary = root.join("bin/jumper");
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
        env::temp_dir().join(format!("jumper-main-{name}-{}-{nanos}", std::process::id()))
    }

    #[test]
    fn parse_args_accepts_lowercase_direct_target() {
        let options = parse_args(["b1".to_owned()].into_iter()).expect("parse direct target");

        assert_eq!(options.command, Command::Jump);
        assert_eq!(options.target.as_deref(), Some("b1"));
    }

    #[test]
    fn release_asset_names_cover_supported_platforms() {
        assert_eq!(
            release_asset_name_for("linux", "x86_64"),
            Some("jumper-linux-x86_64.tar.gz")
        );
        assert_eq!(
            release_asset_name_for("linux", "aarch64"),
            Some("jumper-linux-aarch64.tar.gz")
        );
        assert_eq!(
            release_asset_name_for("macos", "x86_64"),
            Some("jumper-macos-x86_64.tar.gz")
        );
        assert_eq!(
            release_asset_name_for("macos", "aarch64"),
            Some("jumper-macos-aarch64.tar.gz")
        );
        assert_eq!(release_asset_name_for("windows", "x86_64"), None);
    }
}
