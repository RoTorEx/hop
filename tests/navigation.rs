use std::sync::atomic::{AtomicUsize, Ordering};
use std::{fs, path::PathBuf, process::Command, time::SystemTime};

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        static NEXT_WORKSPACE: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "hop-navigation-{}-{}-{}",
            std::process::id(),
            NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
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
        report.contains(&format!("8  project  {}", destination.display())),
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
            && !personal.contains("departed"),
        "{personal}"
    );
    let unavailable_root = workspace.run(&["A0"]);
    assert!(
        !unavailable_root.status.success(),
        "{unavailable_root:?}\n{personal}"
    );
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
            "--scope requires personal, work or archive",
        ),
        (
            &["list", "--scope", "unknown"],
            "--scope requires personal, work or archive",
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

#[test]
fn routing_files_define_scopes_and_share_validation() {
    let workspace = Workspace::new();
    let config = workspace.0.join(".toolbelt/hop/config.toml");
    let routing = config.with_file_name("routing");
    fs::create_dir_all(&routing).unwrap();
    let personal = workspace.0.join("personal");
    let work = workspace.0.join("work");
    fs::create_dir_all(personal.join("local/.git")).unwrap();
    fs::create_dir_all(work.join("company/.git")).unwrap();
    let settings = format!(
        "version = 5\nscan_root = {}\n",
        serde_json::json!(workspace.0)
    );
    fs::write(&config, &settings).unwrap();
    let section = |root: &std::path::Path, item: &str| {
        format!(
            "version = 5\n[[sections]]\nroot = {}\njump_root = true\nitems = [\"{item}\"]\n",
            serde_json::json!(root)
        )
    };
    let personal_file = routing.join("personal.toml");
    let work_file = routing.join("work.toml");
    let personal_routes = section(&personal, "local");
    let work_routes = section(&work, "company");
    fs::write(&personal_file, &personal_routes).unwrap();
    let missing = workspace.run(&["list"]);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("work.toml"));
    fs::write(&work_file, &work_routes).unwrap();

    let personal_view = workspace.run(&["list"]);
    assert!(personal_view.status.success(), "{personal_view:?}");
    let personal_view = String::from_utf8_lossy(&personal_view.stderr);
    assert!(personal_view.contains("local") && !personal_view.contains("company"));
    let work_view = workspace.run(&["A0", "--scope", "work"]);
    assert!(work_view.status.success(), "{work_view:?}");
    assert_eq!(
        String::from_utf8_lossy(&work_view.stdout).trim(),
        work.to_str().unwrap()
    );
    let jump = workspace.run(&["--record-jump", work.to_str().unwrap()]);
    assert!(jump.status.success());
    assert!(
        String::from_utf8_lossy(&workspace.run(&["stats"]).stderr)
            .contains("Total successful jumps: 1")
    );

    let scan = workspace.run(&["scan"]);
    assert!(scan.status.success(), "{scan:?}");
    assert!(String::from_utf8_lossy(&scan.stderr).contains("Found 2 repositories, 0 candidates"));
    assert!(workspace.run(&["config"]).status.success());
    assert_eq!(fs::read_to_string(&personal_file).unwrap(), personal_routes);
    assert_eq!(fs::read_to_string(&work_file).unwrap(), work_routes);
    assert_eq!(fs::read_to_string(&config).unwrap(), settings);

    fs::write(
        &work_file,
        section(&personal.join("local/nested"), "component"),
    )
    .unwrap();
    let conflict = workspace.run(&["list", "--scope", "work"]);
    assert!(!conflict.status.success());
    assert!(String::from_utf8_lossy(&conflict.stderr).contains("overlap across sections"));
}

#[test]
fn archive_scope_is_optional_and_counts_follow_a_moved_destination() {
    let workspace = Workspace::new();
    let routing = workspace.0.join(".toolbelt/hop/routing");
    fs::create_dir_all(&routing).unwrap();
    let old_group = workspace.0.join("tools");
    let new_group = workspace.0.join("dev-tools");
    let archive = workspace.0.join("archive");
    fs::create_dir_all(old_group.join("hop/.git")).unwrap();
    fs::create_dir_all(&archive).unwrap();
    let personal = |root: &std::path::Path| {
        format!(
            "version = 5\n[[sections]]\nroot = {}\nitems = [\"hop\"]\n",
            serde_json::json!(root)
        )
    };
    fs::write(routing.join("personal.toml"), personal(&old_group)).unwrap();
    fs::write(routing.join("work.toml"), "version = 5\n").unwrap();

    // Routing written before the archive scope existed stays valid.
    assert!(workspace.run(&["list"]).status.success());
    let empty = workspace.run(&["list", "--scope", "archive"]);
    assert!(!empty.status.success());
    assert!(String::from_utf8_lossy(&empty.stderr).contains("No available archive projects"));

    fs::write(
        routing.join("archive.toml"),
        format!(
            "version = 5\n[[sections]]\nroot = {}\njump_root = true\nitems = []\n",
            serde_json::json!(archive)
        ),
    )
    .unwrap();
    let archived = workspace.run(&["A0", "--scope", "archive"]);
    assert!(archived.status.success(), "{archived:?}");
    assert_eq!(
        String::from_utf8_lossy(&archived.stdout).trim(),
        archive.to_str().unwrap()
    );
    let personal_view = workspace.run(&["list"]);
    assert!(!String::from_utf8_lossy(&personal_view.stderr).contains("archive"));

    let record = |path: &std::path::Path| {
        let output = workspace.run(&["--record-jump", path.to_str().unwrap()]);
        assert!(output.status.success(), "{output:?}");
    };
    record(&old_group.join("hop"));
    record(&old_group.join("hop"));

    fs::rename(&old_group, &new_group).unwrap();
    fs::write(routing.join("personal.toml"), personal(&new_group)).unwrap();
    record(&new_group.join("hop"));

    // The name resolves to the current path from any scope.
    let by_name = workspace.run(&["hop", "--scope", "archive"]);
    assert!(by_name.status.success(), "{by_name:?}");
    assert_eq!(
        String::from_utf8_lossy(&by_name.stdout).trim(),
        new_group.join("hop").to_str().unwrap()
    );
    let unknown = workspace.run(&["nothing-here"]);
    assert!(!unknown.status.success() && unknown.stdout.is_empty());

    let stats = workspace.run(&["stats"]);
    let report = String::from_utf8_lossy(&stats.stderr);
    assert!(
        report.contains(&format!("3  hop  {}", new_group.join("hop").display())),
        "{report}"
    );
    let frequent = workspace.run(&["list", "--frequent"]);
    assert!(
        String::from_utf8_lossy(&frequent.stderr).contains("3 jumps"),
        "{frequent:?}"
    );
    let history = fs::read_to_string(workspace.0.join(".toolbelt/hop/history.toml")).unwrap();
    assert!(history.contains("version = 2"), "{history}");
    assert_eq!(history.matches("[[projects]]").count(), 1, "{history}");
}
