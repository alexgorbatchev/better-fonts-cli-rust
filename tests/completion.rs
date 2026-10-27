use better_fonts_cli::cli::{self, Environment};
use cli_command_kit::Mode;
use std::collections::BTreeMap;

#[tokio::test]
async fn completion_scripts_cover_native_commands_and_optional_descriptions() {
    let directory = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let environment = Environment {
        mode: Mode::Agent,
        home: None,
        config: directory.path().join("config"),
        cache: directory.path().join("cache"),
        release: BTreeMap::new(),
    };
    for shell in ["bash", "zsh", "fish", "powershell"] {
        let mut output = Vec::new();
        cli::execute(
            ["better-fonts", "completion", "script", shell],
            &environment,
            &mut output,
        )
        .await
        .unwrap();
        let script = String::from_utf8(output).unwrap();
        assert!(script.contains("better-fonts"));
        assert!(script.contains("patch"));
        assert!(script.contains("no-restart"));
        let mut output = Vec::new();
        cli::execute(
            [
                "better-fonts",
                "completion",
                "script",
                shell,
                "--no-descriptions",
            ],
            &environment,
            &mut output,
        )
        .await
        .unwrap();
        assert!(
            !String::from_utf8(output)
                .unwrap()
                .contains("Apply the configured font")
        );
    }
    assert!(!environment.config.exists());
    assert!(
        cli::execute(
            ["better-fonts", "completion", "script", "unknown"],
            &environment,
            &mut Vec::new()
        )
        .await
        .is_err()
    );
}
