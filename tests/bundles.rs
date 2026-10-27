use better_fonts_cli::{
    Error,
    app::{App, Apply, Driver},
    fuses,
    process::{Runner, SystemRunner},
};
use std::{
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
};
use tokio_util::sync::CancellationToken;

struct Tools {
    fail: Option<&'static str>,
    calls: std::sync::Mutex<Vec<(String, Vec<OsString>)>>,
}
impl Runner for Tools {
    fn output(
        &self,
        program: &OsStr,
        args: &[OsString],
        _: &CancellationToken,
    ) -> better_fonts_cli::Result<Vec<u8>> {
        let name = program.to_string_lossy().to_string();
        self.calls
            .lock()
            .unwrap()
            .push((name.clone(), args.to_vec()));
        if self.fail == Some(&name) {
            return Err(Error::Process {
                program: name,
                message: "fixture failure".into(),
            });
        }
        if name == "lipo" {
            return Ok(b"arm64 x86_64\n".to_vec());
        }
        if name == "clang" {
            let i = args.iter().position(|a| a == "-o").unwrap();
            fs::write(&args[i + 1], b"native compiled artifact").unwrap();
        }
        Ok(Vec::new())
    }
}
fn native(path: &Path) -> App {
    fs::create_dir_all(path.join("Contents/MacOS")).unwrap();
    fs::write(
        path.join("Contents/MacOS/TestLauncher"),
        b"original executable",
    )
    .unwrap();
    let mut dict = plist::Dictionary::new();
    dict.insert(
        "CFBundleExecutable".into(),
        plist::Value::String("TestLauncher".into()),
    );
    plist::Value::Dictionary(dict)
        .to_file_xml(path.join("Contents/Info.plist"))
        .unwrap();
    App {
        id: "demo".into(),
        name: "Demo".into(),
        path: path.into(),
        process_name: "Demo\"name".into(),
        driver: Driver::NativeHook,
        asar_path: PathBuf::from("app.asar"),
        preload_path: "preload.js".into(),
    }
}
#[test]
fn native_compile_and_sign_failures_leave_original_bundle_unmodified() {
    for fail in ["clang", "codesign"] {
        let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
        let app = native(&dir.path().join("Demo.app"));
        let runner = Tools {
            fail: Some(fail),
            calls: Default::default(),
        };
        assert!(
            app.apply(
                &Apply {
                    font: "Different Font",
                    patch: true,
                    restart: false,
                    dry_run: false,
                    cache: &dir.path().join("cache")
                },
                &runner,
                &CancellationToken::new()
            )
            .is_err()
        );
        assert_eq!(
            fs::read(app.path.join("Contents/MacOS/TestLauncher")).unwrap(),
            b"original executable"
        );
        assert!(!app.path.join("Contents/MacOS/TestLauncher.orig").exists());
        assert!(!app.path.join("Contents/MacOS/libfonthook.dylib").exists());
    }
}
#[test]
fn native_patch_unpatch_preserves_backup_reports_font_and_uses_exact_process_arguments() {
    let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let app = native(&dir.path().join("Demo.app"));
    let cache = dir.path().join("cache");
    let runner = Tools {
        fail: None,
        calls: Default::default(),
    };
    let token = CancellationToken::new();
    app.apply(
        &Apply {
            font: "Custom \"Font",
            patch: true,
            restart: true,
            dry_run: false,
            cache: &cache,
        },
        &runner,
        &token,
    )
    .unwrap();
    assert_eq!(app.status().unwrap().font.as_deref(), Some("Custom \"Font"));
    assert_eq!(
        fs::read(app.path.join("Contents/MacOS/TestLauncher.orig")).unwrap(),
        b"original executable"
    );
    let calls = runner.calls.lock().unwrap();
    let clang = calls
        .iter()
        .find(|(name, args)| name == "clang" && args.iter().any(|a| a == "-dynamiclib"))
        .unwrap();
    assert_eq!(clang.1.iter().filter(|a| *a == "-arch").count(), 2);
    let quit = calls.iter().find(|(name, _)| name == "osascript").unwrap();
    assert_eq!(quit.1.last().unwrap(), "Demo\"name");
    drop(calls);
    app.apply(
        &Apply {
            font: "unused",
            patch: false,
            restart: false,
            dry_run: false,
            cache: &cache,
        },
        &runner,
        &token,
    )
    .unwrap();
    assert_eq!(
        fs::read(app.path.join("Contents/MacOS/TestLauncher")).unwrap(),
        b"original executable"
    );
    assert!(!app.status().unwrap().patched);
    assert!(!app.path.join("Contents/MacOS/TestLauncher.orig").exists());
}
#[test]
fn dry_run_and_cancel_never_compile_or_publish() {
    let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let app = native(&dir.path().join("Demo.app"));
    let runner = Tools {
        fail: None,
        calls: Default::default(),
    };
    let token = CancellationToken::new();
    app.apply(
        &Apply {
            font: "Font",
            patch: true,
            restart: true,
            dry_run: true,
            cache: &dir.path().join("cache"),
        },
        &runner,
        &token,
    )
    .unwrap();
    assert!(runner.calls.lock().unwrap().is_empty());
    token.cancel();
    assert!(matches!(
        app.apply(
            &Apply {
                font: "Font",
                patch: true,
                restart: false,
                dry_run: false,
                cache: &dir.path().join("cache")
            },
            &runner,
            &token
        ),
        Err(Error::Cancelled)
    ));
}

#[test]
fn custom_app_without_process_name_patches_without_attempting_an_empty_restart() {
    let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let mut app = native(&dir.path().join("Demo.app"));
    app.process_name.clear();
    let runner = Tools {
        fail: None,
        calls: Default::default(),
    };
    app.apply(
        &Apply {
            font: "Custom",
            patch: true,
            restart: true,
            dry_run: false,
            cache: &dir.path().join("cache"),
        },
        &runner,
        &CancellationToken::new(),
    )
    .unwrap();
    assert!(app.status().unwrap().patched);
    let calls = runner.calls.lock().unwrap();
    assert!(
        !calls
            .iter()
            .any(|(program, _)| program == "open" || program == "osascript")
    );
}
fn wire(version: u8, len: u8, state: u8) -> Vec<u8> {
    let mut bytes = fuses::SENTINEL.to_vec();
    bytes.extend([version, len, 48, 49, 48, 49, state]);
    bytes
}
#[test]
fn universal_fuse_validation_precedes_any_write_and_version_length_are_checked() {
    let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let path = dir.path().join("Electron");
    let mut bytes = vec![b'x'; 65520];
    bytes.extend(wire(1, 5, 49));
    bytes.extend(wire(9, 5, 49));
    fs::write(&path, &bytes).unwrap();
    assert!(fuses::disable(&path).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    bytes.truncate(65520 + 39);
    bytes.extend(wire(1, 5, 49));
    fs::write(&path, &bytes).unwrap();
    fuses::disable(&path).unwrap();
    assert!(fuses::is_disabled(&path).unwrap());
    fs::write(&path, wire(1, 4, 49)).unwrap();
    assert!(fuses::disable(&path).is_err());
}
#[cfg(unix)]
#[test]
fn real_subprocess_captures_errors_without_pipe_deadlock_and_honors_cancellation() {
    let runner = SystemRunner::default();
    let token = CancellationToken::new();
    let bytes = runner
        .output(
            OsStr::new("sh"),
            &["-c".into(), "printf output; printf diagnostic >&2".into()],
            &token,
        )
        .unwrap();
    assert_eq!(bytes, b"output");
    assert!(
        runner
            .output(
                OsStr::new("sh"),
                &["-c".into(), "printf failure >&2; exit 4".into()],
                &token
            )
            .unwrap_err()
            .to_string()
            .contains("failure")
    );
    let cancel = token.clone();
    let worker = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(40));
        cancel.cancel();
    });
    let start = std::time::Instant::now();
    assert!(matches!(
        runner.output(
            OsStr::new("sh"),
            &["-c".into(), "exec sleep 30".into()],
            &token
        ),
        Err(Error::Cancelled)
    ));
    worker.join().unwrap();
    assert!(start.elapsed() < std::time::Duration::from_secs(2));
}
