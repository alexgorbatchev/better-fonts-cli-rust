use better_fonts_cli::cli::{self, Environment};
use cli_command_kit::Mode;
use std::{collections::BTreeMap, fs, io};
#[tokio::test]
async fn offline_metadata_commands_have_exact_output_no_config_writes_and_write_errors() {
    let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    for mode in [Mode::Human, Mode::Agent] {
        let environment = Environment {
            mode,
            home: Some(dir.path().to_owned()),
            config: dir.path().join("config.toml"),
            cache: dir.path().join("cache"),
            release: BTreeMap::new(),
        };
        let mut output = Vec::new();
        cli::execute(["better-fonts", "skill"], &environment, &mut output)
            .await
            .unwrap();
        assert_eq!(output, cli::SKILL.as_bytes());
        assert!(!environment.config.exists());
        output.clear();
        cli::execute(["better-fonts", "--version"], &environment, &mut output)
            .await
            .unwrap();
        assert_eq!(output, b"0.1.0\n");
        assert!(
            cli::execute(
                ["better-fonts", "skill", "extra"],
                &environment,
                &mut output
            )
            .await
            .is_err()
        );
        let mut closed = Closed;
        assert!(
            cli::execute(["better-fonts", "skill"], &environment, &mut closed)
                .await
                .is_err()
        );
    }
}
struct Closed;
impl io::Write for Closed {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[tokio::test]
async fn app_records_are_parseable_and_missing_explicit_targets_fail() {
    let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    for mode in [Mode::Human, Mode::Agent] {
        let environment = Environment {
            mode,
            home: Some(dir.path().into()),
            config: dir.path().join("config.toml"),
            cache: dir.path().join("cache"),
            release: BTreeMap::new(),
        };
        let mut output = Vec::new();
        cli::execute(["better-fonts", "app", "list"], &environment, &mut output)
            .await
            .unwrap();
        assert!(String::from_utf8(output.clone()).unwrap().contains("Slack"));
        if mode == Mode::Agent {
            for line in String::from_utf8(output).unwrap().lines() {
                let _: serde_json::Value = serde_json::from_str(line).unwrap();
            }
        }
        let mut output = Vec::new();
        cli::execute(
            ["better-fonts", "app", "status", "paseo"],
            &environment,
            &mut output,
        )
        .await
        .unwrap();
        assert!(String::from_utf8(output).unwrap().contains("installed"));
        assert!(
            cli::execute(
                ["better-fonts", "app", "patch", "missing-not-real"],
                &environment,
                &mut Vec::new()
            )
            .await
            .is_err()
        );
    }
}
#[test]
fn help_paths_include_alerts_and_native_tree_rendering() {
    for mode in [Mode::Human, Mode::Agent] {
        for path in [
            vec![],
            vec!["app"],
            vec!["app", "patch"],
            vec!["app", "unpatch"],
            vec!["app", "status"],
            vec!["app", "list"],
            vec!["config"],
            vec!["config", "path"],
            vec!["config", "show"],
            vec!["config", "init"],
            vec!["self"],
            vec!["self", "upgrade"],
            vec!["skill"],
            vec!["help"],
            vec!["help", "app"],
        ] {
            let generated = path.first() == Some(&"help");
            let mut args = vec!["better-fonts"];
            args.extend(path);
            if !generated {
                args.push("--help");
            }
            let error = cli::command(mode)
                .try_get_matches_from(args.clone())
                .unwrap_err();
            assert!(!error.use_stderr(), "{args:?}: {error}");
            assert_eq!(
                error
                    .to_string()
                    .starts_with("ALERT: Agents must read `AGENT=1 better-fonts skill`"),
                mode == Mode::Agent
            );
        }
    }
}
#[test]
fn exit_classification_and_explicit_release_policy_preserve_typed_failures() {
    assert_eq!(cli::exit_status(&better_fonts_cli::Error::Cancelled), 130);
    assert_eq!(
        cli::exit_status(&better_fonts_cli::Error::Committed("launch failed".into())),
        6
    );
    let parse = cli::command(Mode::Human)
        .try_get_matches_from(["better-fonts", "no-such-command"])
        .unwrap_err();
    assert_eq!(cli::exit_status(&parse.into()), 2);
    let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let environment = Environment {
        mode: Mode::Human,
        home: None,
        config: dir.path().join("config"),
        cache: dir.path().join("cache"),
        release: BTreeMap::from([
            (
                "BETTER_FONTS_RUST_RELEASE_BASE_URL".into(),
                "http://127.0.0.1:1".into(),
            ),
            ("BETTER_FONTS_RUST_RELEASE_OWNER".into(), "owner".into()),
            ("BETTER_FONTS_RUST_RELEASE_REPO".into(), "repo".into()),
        ]),
    };
    assert!(cli::updater(&environment).unwrap().channel.is_some());
}
#[tokio::test]
async fn config_commands_roundtrip_and_home_paths_are_folded() {
    let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let environment = Environment {
        mode: Mode::Agent,
        home: Some(dir.path().to_owned()),
        config: dir.path().join("config.toml"),
        cache: dir.path().join("cache"),
        release: BTreeMap::new(),
    };
    let mut output = Vec::new();
    cli::execute(
        ["better-fonts", "config", "path"],
        &environment,
        &mut output,
    )
    .await
    .unwrap();
    assert_eq!(
        String::from_utf8(output.clone()).unwrap(),
        "~/config.toml\n"
    );
    assert!(!environment.config.exists());
    cli::execute(
        ["better-fonts", "config", "init"],
        &environment,
        &mut output,
    )
    .await
    .unwrap();
    assert!(environment.config.exists());
    fs::write(&environment.config, "font='Custom'\nrestart=false\n").unwrap();
    output.clear();
    cli::execute(
        ["better-fonts", "config", "show"],
        &environment,
        &mut output,
    )
    .await
    .unwrap();
    assert!(String::from_utf8(output).unwrap().contains("Custom"));
}
#[tokio::test]
async fn updater_defaults_to_typed_unpublished_error_and_validates_complete_channel() {
    let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let mut environment = Environment {
        mode: Mode::Agent,
        home: None,
        config: dir.path().join("config"),
        cache: dir.path().join("cache"),
        release: BTreeMap::new(),
    };
    let error = cli::execute(
        ["better-fonts", "self", "upgrade"],
        &environment,
        &mut Vec::new(),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        better_fonts_cli::Error::Dependency(cli_deps::Error::Unpublished)
    ));
    environment
        .release
        .insert("BETTER_FONTS_RUST_RELEASE_OWNER".into(), "example".into());
    assert!(
        cli::execute(
            ["better-fonts", "self", "upgrade"],
            &environment,
            &mut Vec::new()
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("all three")
    );
    assert!(!environment.config.exists());
}
#[test]
fn parsing_rejects_invalid_driver_and_conflicting_flags_before_any_side_effect() {
    let mode = Mode::Agent;
    for args in [
        vec!["better-fonts", "app", "patch", "--driver", "wrong"],
        vec!["better-fonts", "app", "patch", "--restart", "--no-restart"],
    ] {
        assert!(cli::command(mode).try_get_matches_from(args).is_err());
    }
    let matches = cli::command(mode)
        .try_get_matches_from([
            "better-fonts",
            "app",
            "patch",
            "--restart=false",
            "--font",
            "Custom",
        ])
        .unwrap();
    let leaf = matches.subcommand().unwrap().1.subcommand().unwrap().1;
    assert_eq!(leaf.get_one::<bool>("restart"), Some(&false));
}
