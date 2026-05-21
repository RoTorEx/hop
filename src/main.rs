use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, ExitCode, Stdio};

use jumper::{APP_NAME, ChoiceParseError, Sector, discover_projects, group_projects, parse_choice};

const VERSION: &str = env!("CARGO_PKG_VERSION");

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
        Command::Jump => run_jump(options),
    }
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
            _ => {
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
    jumper --shell-init
    jumper --version

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
