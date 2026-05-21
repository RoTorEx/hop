use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitCode, Stdio};

use jumper::{APP_NAME, ChoiceParseError, Sector, discover_projects, group_projects, parse_choice};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASE_BASE_URL: &str = "https://github.com/RoTorEx/jumper/releases/latest/download";

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
            print_help();
            ExitCode::SUCCESS
        }
        Command::Version => {
            println!("{APP_NAME} {VERSION}");
            ExitCode::SUCCESS
        }
        Command::ShellInit => {
            print_shell_init();
            ExitCode::SUCCESS
        }
        Command::Update => run_update(options.color),
        Command::Jump => run_jump(options),
    }
}

fn run_update(color: bool) -> ExitCode {
    let release_asset = match release_asset_name() {
        Some(asset) => asset,
        None => {
            return fail(
                "jumper update currently supports Linux release builds only.",
                color,
            );
        }
    };
    let release_url = format!("{RELEASE_BASE_URL}/{release_asset}");

    let current_exe = match env::current_exe() {
        Ok(path) => path,
        Err(error) => return fail(&format!("Cannot locate current executable: {error}"), color),
    };

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
    if let Err(message) = download_release(&release_url, &archive) {
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

    if let Err(error) = install_updated_binary(&updated, &current_exe) {
        cleanup_dir(&temp_dir);
        return fail(
            &format!("Cannot update {}: {error}", current_exe.display()),
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
    let root = match options.root {
        Some(root) => root,
        None => match env::var_os("HOME") {
            Some(home) => PathBuf::from(home),
            None => return fail("HOME is not set; pass --root <dir>", options.color),
        },
    };

    let projects = match discover_projects(&root) {
        Ok(projects) => projects,
        Err(error) => {
            return fail(
                &format!("Cannot scan {}: {error}", root.display()),
                options.color,
            );
        }
    };

    if projects.is_empty() {
        return fail("No git projects found under the scan root.", options.color);
    }

    let sectors = group_projects(&root, projects);

    if let Some(target) = options.target {
        return choose_target(&sectors, &target, options.copy_path, options.color);
    }

    render(&sectors, options.color);
    prompt_loop(&sectors, options.copy_path, options.color)
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
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => Some("jumper-linux-x86_64.tar.gz"),
        ("linux", "aarch64") => Some("jumper-linux-aarch64.tar.gz"),
        _ => None,
    }
}

fn download_release(url: &str, destination: &Path) -> Result<(), String> {
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

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> io::Result<()> {
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
            "-V" | "--version" => command = Command::Version,
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
            "update" if target.is_none() => {
                if command != Command::Jump {
                    return Err(format!("unexpected argument: {arg}"));
                }
                command = Command::Update;
            }
            _ => {
                if command == Command::Update {
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

fn print_help() {
    println!(
        "{APP_NAME} {VERSION}

Tiny interactive project navigator for shells on local machines, VMs, and VPS hosts.

USAGE:
    jumper [<target>] [--copy-path] [--root <dir>] [--no-color]
    jumper update
    jumper --shell-init
    jumper --version

COMMANDS:
    update           Update this executable from the latest GitHub release

OPTIONS:
    --copy-path      Copy the selected path instead of printing it
    --root <dir>      Scan a directory instead of $HOME
    --no-color        Disable ANSI color output
    --shell-init      Print bash/zsh integration for the j command
    -V, --version     Print version
    -h, --help        Print help

The interactive UI writes to stderr. Jump mode prints only the selected project
path to stdout, so a shell wrapper can safely cd into it. Copy mode writes no
stdout and copies the selected project path to the clipboard."
    );
}

fn print_shell_init() {
    println!(
        r#"# x-cli-jumper
export PATH="$HOME/.x-cli-jumper:$PATH"

j() {{
    local d
    d="$(jumper "$@")" && [ -n "$d" ] && cd "$d"
}}"#
    );
}
