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
    assert!(refresh().status.success());

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

    assert!(refresh().status.success());
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
