use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

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
    render(&sectors, options.color);
    prompt_loop(&sectors, options.color)
}

fn prompt_loop(sectors: &[Sector], color: bool) -> ExitCode {
    loop {
        eprint!(
            "  {} {} {}: ",
            paint(color, BLUE, ">"),
            paint(color, BOLD, "jump to"),
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

        let Some(sector) = sectors.get(choice.sector_index) else {
            warn("No such sector", color);
            continue;
        };
        let Some(path) = sector.paths.get(choice.project_index) else {
            warn("No such project", color);
            continue;
        };

        println!("{}", path.display());
        return ExitCode::SUCCESS;
    }
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut command = Command::Jump;
    let mut root = None;
    let mut color = colors_enabled();
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => command = Command::Help,
            "-V" | "--version" => command = Command::Version,
            "--shell-init" => command = Command::ShellInit,
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
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    Ok(Options {
        command,
        root,
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
    jumper [--root <dir>] [--no-color]
    jumper --shell-init
    jumper --version

OPTIONS:
    --root <dir>      Scan a directory instead of $HOME
    --no-color        Disable ANSI color output
    --shell-init      Print bash/zsh integration for the j command
    -V, --version     Print version
    -h, --help        Print help

The interactive UI writes to stderr. The selected project path is the only stdout
output, so a shell wrapper can safely cd into it."
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
