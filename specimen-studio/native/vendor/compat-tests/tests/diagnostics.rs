use std::{
    env, fs,
    path::Path,
    process::{Command, Output, Stdio},
};

fn run(cargo: &str, root: &Path, args: &[&str]) -> Output {
    Command::new(cargo)
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .expect("execute pinned Cargo")
}

#[test]
fn invalid_derives_report_intended_errors() {
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let compiler = Command::new(rustc).arg("--version").output().unwrap();
    let identity = String::from_utf8(compiler.stdout).unwrap();
    assert!(
        identity.starts_with("rustc 1.99.1 "),
        "review diagnostics on compiler change: {identity}"
    );
    let cargo = env::var("CARGO").expect("Cargo runner");
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let target =
        env::var_os("CARGO_TARGET_DIR").expect("use an isolated absolute CARGO_TARGET_DIR");
    let root = Path::new(&target).join("diagnostic-fixture");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"
[package]
name = "studio-gtk-negative-fixture"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
glib-macros = "=0.18.5"
gtk3-macros = "=0.18.2"
glib = {{ version = "=0.18.5", optional = true }}
gtk = {{ version = "=0.18.2", optional = true }}
[patch.crates-io]
glib = {{ path = {:?} }}
glib-macros = {{ path = {:?} }}
gtk3-macros = {{ path = {:?} }}
"#,
            vendor.join("glib-0.18.5"),
            vendor.join("glib-macros-0.18.5"),
            vendor.join("gtk3-macros-0.18.2")
        ),
    )
    .unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    let lock = run(&cargo, &root, &["generate-lockfile", "--offline"]);
    assert!(
        lock.status.success(),
        "{}",
        String::from_utf8_lossy(&lock.stderr)
    );
    let graph = run(
        &cargo,
        &root,
        &[
            "metadata",
            "--offline",
            "--locked",
            "--all-features",
            "--format-version",
            "1",
        ],
    );
    assert!(
        graph.status.success(),
        "{}",
        String::from_utf8_lossy(&graph.stderr)
    );
    let graph: serde_json::Value = serde_json::from_slice(&graph.stdout).unwrap();
    for (name, version) in [
        ("glib", "0.18.5"),
        ("glib-macros", "0.18.5"),
        ("gtk3-macros", "0.18.2"),
    ] {
        let package = graph["packages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name && p["version"] == version)
            .unwrap();
        assert!(
            package["source"].is_null(),
            "registry substitution: {package}"
        );
        assert_eq!(
            Path::new(package["manifest_path"].as_str().unwrap()),
            vendor.join(format!("{name}-{version}/Cargo.toml"))
        );
    }
    assert!(!graph["packages"].as_array().unwrap().iter().any(|p| [
        "proc-macro-error",
        "proc-macro-error2"
    ]
    .contains(&p["name"].as_str().unwrap())));
    let cases = [
        ("enum_struct", "#[derive(glib::Enum)] only supports enums"),
        (
            "enum_missing_name",
            "#[derive(glib::Enum)] requires #[enum_type(name = \"EnumTypeName\")]",
        ),
        (
            "template_missing",
            "Missing 'template' attribute: derive(CompositeTemplate) requires #[template(...)]",
        ),
        (
            "template_child_malformed",
            "invalid attribute argument type, expected `name = value` list or nothing",
        ),
    ];
    for (case, expected) in cases {
        fs::copy(
            vendor.join(format!("compat-tests/fixtures/{case}.rs")),
            root.join("src/main.rs"),
        )
        .unwrap();
        let output = run(
            &cargo,
            &root,
            &["check", "--offline", "--locked", "--color", "never"],
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        fs::write(root.join(format!("{case}.stderr")), stderr.as_bytes()).unwrap();
        assert!(!output.status.success(), "{case} unexpectedly succeeded");
        assert!(
            stderr.contains(&format!("error: {expected}")),
            "{case}: {stderr}"
        );
        let errors: Vec<_> = stderr
            .lines()
            .filter(|line| {
                line.starts_with("error") && !line.starts_with("error: could not compile")
            })
            .collect();
        assert_eq!(
            errors.len(),
            1,
            "{case}: unexpected extra compiler diagnostic: {stderr}"
        );
        for forbidden in [
            "proc macro panicked",
            "proc-macro derive panicked",
            "unresolved import",
            "failed to resolve",
            "can't find crate",
            "cannot find crate",
            "use of unresolved module",
            "Could not find `gtk`",
        ] {
            assert!(
                !stderr.contains(forbidden),
                "{case}: unexpected failure {forbidden}: {stderr}"
            );
        }
    }
}
