use std::{fs, path::PathBuf, process::Command, time::SystemTime};

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "hop-navigation-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_hop"))
            .args(args)
            .current_dir(&self.0)
            .env("HOME", &self.0)
            .env("USERPROFILE", &self.0)
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn projects_are_discovered_only_on_explicit_config_refresh() {
    let workspace = Workspace::new();
    let projects = workspace.0.join("projects");
    fs::create_dir_all(projects.join("saved-project/.git")).unwrap();
    let refresh = || workspace.run(&["config", "--root", projects.to_str().unwrap()]);
    let refreshed = refresh();
    assert!(refreshed.status.success(), "{refreshed:?}");

    fs::create_dir_all(projects.join("new-project/.git")).unwrap();
    for args in [vec!["list"], vec![], vec!["--version"], vec!["--help"]] {
        // Empty input cancels ordinary navigation after rendering its saved list.
        let output = workspace.run(&args);
        assert_eq!(
            output.status.code(),
            Some(if args.is_empty() { 1 } else { 0 }),
            "{output:?}"
        );
        let ui = String::from_utf8(output.stderr).unwrap();
        assert!(!ui.contains("new-project"), "{ui}");
        assert!(!ui.contains("differs from config"), "{ui}");
    }

    let refreshed = refresh();
    assert!(refreshed.status.success(), "{refreshed:?}");
    let output = workspace.run(&["list"]);
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("new-project")
    );

    fs::remove_dir_all(projects.join("saved-project")).unwrap();
    let output = workspace.run(&["list"]);
    assert!(output.status.success());
    assert!(
        !String::from_utf8(output.stderr)
            .unwrap()
            .contains("saved-project")
    );
}

#[test]
fn navigation_cannot_request_a_recursive_scan() {
    let workspace = Workspace::new();
    for args in [vec!["--root", "."], vec!["list", "--root=."]] {
        let output = workspace.run(&args);
        assert!(!output.status.success());
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains("hop config --root <dir>")
        );
    }
}

#[test]
fn stats_use_recorded_jumps_and_retained_search_events() {
    let workspace = Workspace::new();
    let stats = || workspace.run(&["stats", "--no-color"]);
    let empty = stats();
    assert!(empty.status.success());
    assert!(String::from_utf8_lossy(&empty.stderr).contains("Total successful jumps: 0"));

    let destination = workspace.0.join("project");
    fs::create_dir_all(destination.join(".git")).unwrap();
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                let output = workspace.run(&["--record-jump", destination.to_str().unwrap()]);
                assert!(output.status.success(), "{output:?}");
            });
        }
    });
    // History remains visible after a project disappears.
    fs::remove_dir_all(&destination).unwrap();

    let logs = workspace.0.join(".toolbelt/hop/logs");
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let search = |mode, result, duration, epoch_seconds| {
        serde_json::json!({
            "action": "project_search",
            "result": result,
            "fields": {"mode": mode},
            "duration_ms": duration,
            "epoch_seconds": epoch_seconds,
        })
    };
    fs::write(
        logs.join("events.jsonl.1"),
        format!(
            "{}\n{}\n{}\n",
            search("default", "selected", 500, now),
            search("default", "selected", 20_000, now - 91 * 86400),
            search("frequent", "canceled", 1500, now),
        ),
    )
    .unwrap();

    let output = stats();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    let report = String::from_utf8(output.stderr).unwrap();
    assert!(report.contains("Total successful jumps: 8"), "{report}");
    assert!(
        report.contains(&format!("8  {}", destination.display())),
        "{report}"
    );
    assert!(
        report.contains("default, selected: 1 searches, 0.50 s average"),
        "{report}"
    );
    assert!(
        report.contains("frequent, canceled: 1 searches, 1.50 s average"),
        "{report}"
    );
}

#[test]
fn scopes_use_saved_membership_and_scan_reports_only() {
    let workspace = Workspace::new();
    let projects = workspace.0.join("projects");
    for folder in ["dev-tools/hop", "dev-tools/services/api", "company/repo"] {
        fs::create_dir_all(projects.join(folder).join(".git")).unwrap();
    }
    fs::create_dir_all(projects.join("dev-tools/services/new-component/.git")).unwrap();
    fs::create_dir_all(projects.join("unregistered/.git")).unwrap();

    fs::create_dir_all(projects.join("dev-tools/departed")).unwrap();

    let config_dir = workspace.0.join(".toolbelt/hop");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    let config = format!(
        r#"version = 5
[[sections]]
root = {personal}
items = ["hop", "departed"]
[[sections]]
root = {services}
jump_root = true
items = ["api"]
[[sections]]
root = {work}
scope = "work"
jump_root = true
items = ["repo"]
"#,
        personal = serde_json::json!(projects.join("dev-tools")),
        services = serde_json::json!(projects.join("dev-tools/services")),
        work = serde_json::json!(projects.join("company")),
    );
    fs::write(&config_path, &config).unwrap();

    let personal = workspace.run(&["list"]);
    assert!(personal.status.success(), "{personal:?}");
    assert_eq!(
        personal.stderr,
        workspace.run(&["list", "--scope", "personal"]).stderr
    );
    let personal = String::from_utf8(personal.stderr).unwrap();
    assert!(personal.contains("Projects (personal)") && personal.contains("services"));
    assert!(
        !personal.contains("company")
            && !personal.contains("new-component")
            && !personal.contains("departed")
    );
    assert!(!workspace.run(&["A0"]).status.success());
    let selected = |args: &[&str], directory| {
        let output = workspace.run(args);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            projects.join(directory).to_str().unwrap()
        );
    };
    selected(&["B0"], "dev-tools/services");
    selected(&["A0", "--scope", "work"], "company");
    selected(&["A1", "--scope", "work"], "company/repo");

    let work = workspace.run(&["list", "--frequent", "--scope", "work"]);
    assert!(work.status.success(), "{work:?}");
    let listing = String::from_utf8(work.stderr).unwrap();
    assert!(listing.contains("company") && !listing.contains("dev-tools"));

    for (args, error) in [
        (
            &["list", "--scope"][..],
            "--scope requires personal or work",
        ),
        (
            &["list", "--scope", "unknown"],
            "--scope requires personal or work",
        ),
        (
            &["list", "--scope", "work", "--scope", "personal"],
            "--scope can be specified only once",
        ),
        (
            &["scan", "--scope", "work"],
            "--scope is only supported for jumps and hop list",
        ),
        (&["list", "work"], "unexpected argument: work"),
    ] {
        let result = workspace.run(args);
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(error),
            "{result:?}"
        );
    }

    let scan = workspace.run(&["scan", "--root", "projects"]);
    assert!(scan.status.success() && scan.stdout.is_empty(), "{scan:?}");
    let report = String::from_utf8(scan.stderr).unwrap();
    assert!(report.contains("directories, 5 repositories"), "{report}");
    assert!(
        report.contains("1 candidates, 1 unavailable destinations"),
        "{report}"
    );
    assert!(
        report.contains("Inside ") && report.contains("new-component"),
        "{report}"
    );
    assert_eq!(fs::read_to_string(&config_path).unwrap(), config);
    assert!(workspace.run(&["config"]).status.success());
    assert_eq!(fs::read_to_string(&config_path).unwrap(), config);

    // Listing both the parent project and an inner section would split ownership.
    let conflict = config.replace(r#"["hop", "departed"]"#, r#"["hop", "services"]"#);
    fs::write(&config_path, conflict).unwrap();
    let output = workspace.run(&["list", "--scope", "work"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate jump destination"));

    let cross_scope = config.replace(r#"items = ["api"]"#, r#"items = []"#)
        + &format!(
            "\n[[sections]]\nroot = {}\nscope = \"work\"\njump_root = true\nitems = []\n",
            serde_json::json!(projects.join("dev-tools/services/api"))
        );
    fs::write(&config_path, cross_scope).unwrap();
    let output = workspace.run(&["list"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("overlap across sections"));
}
