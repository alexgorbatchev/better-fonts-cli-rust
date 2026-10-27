use better_fonts_cli::{
    Error, Result,
    app::{App, Apply, Driver},
    process::Runner,
};
use std::{
    ffi::{OsStr, OsString},
    fs,
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio_util::sync::CancellationToken;
fn temp() -> tempfile::TempDir {
    tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap()
}
struct Compiler {
    architecture: &'static [u8],
    empty: bool,
    count: AtomicUsize,
    outputs: Mutex<Vec<OsString>>,
}
impl Runner for Compiler {
    fn output(&self, program: &OsStr, args: &[OsString], _: &CancellationToken) -> Result<Vec<u8>> {
        if program == "lipo" {
            return Ok(self.architecture.into());
        }
        if program == "clang" {
            self.count.fetch_add(1, Ordering::Relaxed);
            let index = args.iter().position(|v| v == "-o").unwrap();
            fs::write(
                &args[index + 1],
                if self.empty {
                    b"".as_slice()
                } else {
                    b"compiled".as_slice()
                },
            )?;
            self.outputs.lock().unwrap().extend_from_slice(args);
        }
        Ok(Vec::new())
    }
}
fn fixture(root: &Path) -> App {
    let path = root.join("Native.app");
    fs::create_dir_all(path.join("Contents/MacOS")).unwrap();
    fs::write(path.join("Contents/MacOS/Native"), b"original").unwrap();
    App {
        id: "native".into(),
        name: "Native".into(),
        path,
        process_name: "Native".into(),
        driver: Driver::NativeHook,
        asar_path: "app.asar".into(),
        preload_path: "preload.js".into(),
    }
}
#[test]
fn cached_compilation_reuses_both_artifacts_and_font_is_encoded_as_c_bytes() {
    let dir = temp();
    let app = fixture(dir.path());
    let compiler = Compiler {
        architecture: b"arm64\n",
        empty: false,
        count: AtomicUsize::new(0),
        outputs: Default::default(),
    };
    let cache = dir.path().join("cache");
    let token = CancellationToken::new();
    let options = Apply {
        font: "A\"\\\n",
        patch: true,
        restart: false,
        dry_run: false,
        cache: &cache,
    };
    app.apply(&options, &compiler, &token).unwrap();
    app.apply(&options, &compiler, &token).unwrap();
    assert_eq!(compiler.count.load(Ordering::Relaxed), 2);
    assert_eq!(app.status().unwrap().font.as_deref(), Some("A\"\\\n"));
    let args = compiler.outputs.lock().unwrap();
    assert!(
        args.iter()
            .any(|a| a == "-DTARGET_FONT_NAME=\"\\101\\042\\134\\012\"")
    );
    fs::remove_file(app.path.join("Contents/MacOS/Native.bak")).unwrap();
    fs::remove_file(app.path.join("Contents/MacOS/.better-fonts.json")).unwrap();
    assert_eq!(app.status().unwrap().font, None);
    app.apply(
        &Apply {
            font: "irrelevant",
            patch: false,
            restart: false,
            dry_run: false,
            cache: &cache,
        },
        &compiler,
        &token,
    )
    .unwrap();
    assert_eq!(
        fs::read(app.path.join("Contents/MacOS/Native")).unwrap(),
        b"original"
    );
}
#[test]
fn unsupported_architecture_empty_artifacts_ambiguous_executables_and_invalid_plists_fail_before_publication()
 {
    for (architecture, empty) in [
        (b"ppc".as_slice(), false),
        (b"\xff".as_slice(), false),
        (b"arm64".as_slice(), true),
    ] {
        let dir = temp();
        let app = fixture(dir.path());
        let compiler = Compiler {
            architecture,
            empty,
            count: AtomicUsize::new(0),
            outputs: Default::default(),
        };
        assert!(
            app.apply(
                &Apply {
                    font: "Font",
                    patch: true,
                    restart: false,
                    dry_run: false,
                    cache: &dir.path().join("cache")
                },
                &compiler,
                &CancellationToken::new()
            )
            .is_err()
        );
        assert_eq!(
            fs::read(app.path.join("Contents/MacOS/Native")).unwrap(),
            b"original"
        );
    }
    let dir = temp();
    let app = fixture(dir.path());
    fs::write(app.path.join("Contents/MacOS/Other"), b"other").unwrap();
    assert!(app.status().is_err());
    let mut dict = plist::Dictionary::new();
    dict.insert(
        "CFBundleExecutable".into(),
        plist::Value::String("../evil".into()),
    );
    plist::Value::Dictionary(dict)
        .to_file_binary(app.path.join("Contents/Info.plist"))
        .unwrap();
    assert!(app.status().is_err());
}
#[cfg(unix)]
#[test]
fn native_artifact_symlinks_cannot_write_outside_the_staged_bundle() {
    let dir = temp();
    let app = fixture(dir.path());
    let outside = dir.path().join("outside");
    fs::write(&outside, b"preserve me").unwrap();
    std::os::unix::fs::symlink(&outside, app.path.join("Contents/MacOS/libfonthook.dylib"))
        .unwrap();
    assert!(app.status().is_err());
    assert_eq!(fs::read(outside).unwrap(), b"preserve me");
}
#[test]
fn app_lock_rejects_a_second_writer_and_releases_on_drop() {
    use sha2::{Digest, Sha256};
    let dir = temp();
    let app = fixture(dir.path());
    let cache = dir.path().join("independent-cache");
    let key = format!(
        "{:x}",
        Sha256::digest(
            fs::canonicalize(&app.path)
                .unwrap()
                .as_os_str()
                .as_encoded_bytes()
        )
    );
    let lock = fs::File::create(dir.path().join(format!(".better-fonts-{key}.lock"))).unwrap();
    lock.lock().unwrap();
    let compiler = Compiler {
        architecture: b"arm64",
        empty: false,
        count: AtomicUsize::new(0),
        outputs: Default::default(),
    };
    let options = Apply {
        font: "Font",
        patch: true,
        restart: false,
        dry_run: false,
        cache: &cache,
    };
    assert!(
        app.apply(&options, &compiler, &CancellationToken::new())
            .unwrap_err()
            .to_string()
            .contains("busy")
    );
    drop(lock);
    app.apply(&options, &compiler, &CancellationToken::new())
        .unwrap();
}
#[cfg(unix)]
#[test]
fn special_bundle_files_fail_without_blocking_or_touching_originals() {
    let dir = temp();
    let app = fixture(dir.path());
    let fifo = app.path.join("fifo");
    let name = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: the C string is NUL terminated, and the fixture path is owned by this test.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let compiler = Compiler {
        architecture: b"arm64",
        empty: false,
        count: AtomicUsize::new(0),
        outputs: Default::default(),
    };
    assert!(matches!(
        app.apply(
            &Apply {
                font: "Font",
                patch: true,
                restart: false,
                dry_run: false,
                cache: &dir.path().join("cache")
            },
            &compiler,
            &CancellationToken::new()
        ),
        Err(Error::Invalid(_))
    ));
    assert_eq!(compiler.count.load(Ordering::Relaxed), 0);
}
