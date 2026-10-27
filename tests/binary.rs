use std::{
    fs,
    process::{Command, Output},
};

fn run(directory: &std::path::Path, arguments: &[&str], agent: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_better-fonts"))
        .env_clear()
        .envs(std::env::vars_os().filter(|(name, _)| name == "LLVM_PROFILE_FILE"))
        .env("PATH", std::env::var_os("PATH").unwrap())
        .env("HOME", directory)
        .env("XDG_CONFIG_HOME", directory.join("config"))
        .env("XDG_CACHE_HOME", directory.join("cache"))
        .env("AGENT", agent)
        .current_dir(directory)
        .args(arguments)
        .output()
        .unwrap()
}

#[test]
fn binary_handles_raw_version_help_relative_configuration_and_errors_once() {
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    for mode in ["0", "1", "true", "yes"] {
        let output = run(directory.path(), &["--version"], mode);
        assert!(output.status.success());
        assert_eq!(output.stdout, b"0.1.0\n");
        assert!(output.stderr.is_empty());
        let output = run(directory.path(), &["--help"], mode);
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .starts_with("ALERT:"),
            mode != "0"
        );
    }
    assert!(!directory.path().join("config").exists());
    let output = run(directory.path(), &["config", "path"], "1");
    assert_eq!(output.stdout, b"~/config/better-fonts/config.toml\n");
    let output = run(
        directory.path(),
        &["--config", "local.toml", "config", "init"],
        "0",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(directory.path().join("local.toml").is_file());
    let output = run(
        directory.path(),
        &["--config", "local.toml", "config", "show"],
        "1",
    );
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Maple Mono")
    );
    fs::write(directory.path().join("local.toml"), "invalid=[").unwrap();
    let output = run(
        directory.path(),
        &["--config", "local.toml", "config", "show"],
        "1",
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr)
            .unwrap()
            .matches("ERR:")
            .count(),
        1
    );
    let output = run(directory.path(), &["app", "patch", "--font", ""], "1");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let output = run(directory.path(), &["self", "upgrade"], "1");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("unpublished")
    );
}

#[test]
fn xdg_fallback_remains_home_dot_config_instead_of_platform_application_support() {
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_better-fonts"))
        .env_clear()
        .envs(std::env::vars_os().filter(|(name, _)| name == "LLVM_PROFILE_FILE"))
        .env("HOME", directory.path())
        .env("AGENT", "1")
        .args(["config", "path"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"~/.config/better-fonts/config.toml\n");
}
