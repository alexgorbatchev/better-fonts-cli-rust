use better_fonts_cli::{
    Error, Result,
    app::{App, Apply, Driver},
    archive::Archive,
    cli::{self, Environment},
    process::Runner,
};
use cli_command_kit::Mode;
use serde_json::json;
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tokio_util::sync::CancellationToken;
fn temp() -> tempfile::TempDir {
    tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap()
}
struct Tools {
    failure: Option<&'static str>,
    remove_stage: bool,
    calls: Mutex<Vec<String>>,
}
impl Runner for Tools {
    fn output(&self, program: &OsStr, args: &[OsString], _: &CancellationToken) -> Result<Vec<u8>> {
        let program = program.to_str().unwrap();
        self.calls.lock().unwrap().push(program.into());
        if self.failure == Some(program) {
            return Err(Error::Process {
                program: program.into(),
                message: "injected native-tool failure".into(),
            });
        }
        if program == "codesign" && self.remove_stage {
            fs::remove_dir_all(args.last().unwrap()).unwrap();
        }
        Ok(Vec::new())
    }
}
fn electron(root: &Path) -> App {
    let path = root.join("Demo.app");
    fs::create_dir_all(path.join("Contents/Resources")).unwrap();
    fs::create_dir_all(path.join("Contents/Frameworks/Electron Framework.framework/Versions/A"))
        .unwrap();
    let header = json!({"files":{"preload.js":{"size":6,"offset":"0"},"other":{"size":4,"offset":"6"},"empty":{"files":{}}}});
    let bytes = serde_json::to_vec(&header).unwrap();
    let padded = (bytes.len() + 3) & !3;
    let mut file = fs::File::create(path.join("Contents/Resources/app.asar")).unwrap();
    for word in [4, padded as u32 + 8, padded as u32 + 4, bytes.len() as u32] {
        file.write_all(&word.to_le_bytes()).unwrap();
    }
    file.write_all(&bytes).unwrap();
    file.write_all(&vec![0; padded - bytes.len()]).unwrap();
    file.write_all(b"code\n\nDATA").unwrap();
    let mut wire = better_fonts_cli::fuses::SENTINEL.to_vec();
    wire.extend([1, 5, b'0', b'0', b'0', b'0', b'1']);
    fs::write(
        path.join("Contents/Frameworks/Electron Framework.framework/Versions/A/Electron Framework"),
        wire,
    )
    .unwrap();
    App {
        id: "demo".into(),
        name: "Demo".into(),
        path,
        process_name: "Demo".into(),
        driver: Driver::Electron,
        asar_path: "app.asar".into(),
        preload_path: "preload.js".into(),
    }
}
#[test]
fn electron_patch_repatch_and_unpatch_preserve_preload_bytes_metadata_and_other_files() {
    let dir = temp();
    let app = electron(dir.path());
    let tools = Tools {
        failure: None,
        remove_stage: false,
        calls: Default::default(),
    };
    let token = CancellationToken::new();
    let cache = dir.path().join("cache");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("app.asar", app.path.join("Contents/Resources/alias")).unwrap();
    }
    let archive_path = app.path.join("Contents/Resources/app.asar");
    #[cfg(target_os = "linux")]
    xattr::set(&archive_path, "user.better-fonts-test", b"preserve").unwrap();
    app.apply(
        &Apply {
            font: "First Font",
            patch: true,
            restart: false,
            dry_run: false,
            cache: &cache,
        },
        &tools,
        &token,
    )
    .unwrap();
    assert_eq!(app.status().unwrap().font.as_deref(), Some("First Font"));
    app.apply(
        &Apply {
            font: "Second Font",
            patch: true,
            restart: false,
            dry_run: false,
            cache: &cache,
        },
        &tools,
        &token,
    )
    .unwrap();
    assert_eq!(app.status().unwrap().font.as_deref(), Some("Second Font"));
    app.apply(
        &Apply {
            font: "unused",
            patch: false,
            restart: false,
            dry_run: false,
            cache: &cache,
        },
        &tools,
        &token,
    )
    .unwrap();
    assert!(!app.status().unwrap().patched);
    let mut archive = Archive::open(&archive_path).unwrap();
    assert_eq!(archive.extract("preload.js").unwrap(), b"code\n\n");
    assert_eq!(archive.extract("other").unwrap(), b"DATA");
    #[cfg(unix)]
    assert_eq!(
        fs::read_link(app.path.join("Contents/Resources/alias")).unwrap(),
        PathBuf::from("app.asar")
    );
    #[cfg(target_os = "linux")]
    assert_eq!(
        xattr::get(&archive_path, "user.better-fonts-test").unwrap(),
        Some(b"preserve".to_vec())
    );
}
#[test]
fn publication_failure_restores_original_and_quit_failure_prevents_publication() {
    for (failure, remove_stage) in [(None, true), (Some("osascript"), false)] {
        let dir = temp();
        let app = electron(dir.path());
        let original = fs::read(app.path.join("Contents/Resources/app.asar")).unwrap();
        let tools = Tools {
            failure,
            remove_stage,
            calls: Default::default(),
        };
        assert!(
            app.apply(
                &Apply {
                    font: "Font",
                    patch: true,
                    restart: true,
                    dry_run: false,
                    cache: &dir.path().join("cache")
                },
                &tools,
                &CancellationToken::new()
            )
            .is_err()
        );
        assert_eq!(
            fs::read(app.path.join("Contents/Resources/app.asar")).unwrap(),
            original
        );
        assert!(!tools.calls.lock().unwrap().iter().any(|v| v == "open"));
    }
}
#[test]
fn restart_failure_is_committed_and_archive_paths_cannot_escape_staging() {
    let dir = temp();
    let mut app = electron(dir.path());
    let tools = Tools {
        failure: Some("open"),
        remove_stage: false,
        calls: Default::default(),
    };
    let error = app
        .apply(
            &Apply {
                font: "Font",
                patch: true,
                restart: true,
                dry_run: false,
                cache: &dir.path().join("cache"),
            },
            &tools,
            &CancellationToken::new(),
        )
        .unwrap_err();
    assert!(matches!(error, Error::Committed(_)));
    assert!(app.status().unwrap().patched);
    app.asar_path = "../outside.asar".into();
    assert!(app.status().is_err());
    app.asar_path = dir.path().join("outside.asar");
    assert!(app.status().is_err());
}
#[tokio::test]
async fn cli_mutations_respect_config_overrides_and_return_batch_failures_once() {
    let dir = temp();
    let app = electron(dir.path());
    let environment = Environment {
        mode: Mode::Agent,
        home: Some(dir.path().into()),
        config: dir.path().join("config.toml"),
        cache: dir.path().join("cache"),
        release: BTreeMap::new(),
    };
    fs::write(&environment.config,format!("font='Global'\napps=['demo']\nrestart=false\n[apps_config.demo]\nfont='Per App'\n[[custom_apps]]\nid='demo'\nname='Demo'\npath={}\nprocess_name='Demo'\npreload_path='preload.js'\n",serde_json::to_string(&app.path).unwrap())).unwrap();
    let tools = Tools {
        failure: None,
        remove_stage: false,
        calls: Default::default(),
    };
    let token = CancellationToken::new();
    let mut output = Vec::new();
    cli::execute_with(
        ["better-fonts", "app", "patch", "--font", "Flag"],
        &environment,
        &tools,
        &mut output,
        &token,
    )
    .await
    .unwrap();
    assert_eq!(app.status().unwrap().font.as_deref(), Some("Flag"));
    assert!(!tools.calls.lock().unwrap().iter().any(|v| v == "open"));
    output.clear();
    cli::execute_with(
        ["better-fonts", "--verbose", "app", "status", "demo"],
        &environment,
        &tools,
        &mut output,
        &token,
    )
    .await
    .unwrap();
    assert!(String::from_utf8(output).unwrap().contains("Flag"));
    cli::execute_with(
        ["better-fonts", "app", "unpatch", "--no-restart"],
        &environment,
        &tools,
        &mut Vec::new(),
        &token,
    )
    .await
    .unwrap();
    assert!(!app.status().unwrap().patched);
    let human = Environment {
        mode: Mode::Human,
        home: environment.home.clone(),
        config: environment.config.clone(),
        cache: environment.cache.clone(),
        release: BTreeMap::new(),
    };
    let mut preview = Vec::new();
    cli::execute_with(
        ["better-fonts", "--verbose", "app", "patch", "--dry-run"],
        &human,
        &tools,
        &mut preview,
        &token,
    )
    .await
    .unwrap();
    let preview = String::from_utf8(preview).unwrap();
    assert!(preview.contains("dry-run font=Per App restart=false ~/Demo.app"));
    assert!(!app.status().unwrap().patched);
    let mut missing = Vec::new();
    cli::execute_with(
        ["better-fonts", "app", "patch", "paseo"],
        &environment,
        &tools,
        &mut missing,
        &token,
    )
    .await
    .unwrap();
    assert!(
        String::from_utf8(missing)
            .unwrap()
            .contains("not-installed")
    );
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert!(matches!(
        cli::execute_with(
            ["better-fonts", "app", "patch"],
            &environment,
            &tools,
            &mut Vec::new(),
            &cancelled
        )
        .await,
        Err(Error::Cancelled)
    ));
    let tools = Tools {
        failure: Some("codesign"),
        remove_stage: false,
        calls: Default::default(),
    };
    let error = cli::execute_with(
        ["better-fonts", "app", "patch"],
        &environment,
        &tools,
        &mut Vec::new(),
        &token,
    )
    .await
    .unwrap_err();
    assert_eq!(cli::exit_status(&error), 1);
    assert_eq!(error.to_string().matches("Demo:").count(), 1);
    let tools = Tools {
        failure: Some("open"),
        remove_stage: false,
        calls: Default::default(),
    };
    let error = cli::execute_with(
        ["better-fonts", "app", "patch", "--restart"],
        &environment,
        &tools,
        &mut Vec::new(),
        &token,
    )
    .await
    .unwrap_err();
    assert_eq!(cli::exit_status(&error), 6);
}
