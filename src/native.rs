use crate::{
    Result, invalid,
    process::{Runner, run},
};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};
use tokio_util::sync::CancellationToken;
const HOOK: &str = include_str!("../assets/font_interpose.m");
const LAUNCHER: &str = include_str!("../assets/wrapper_launcher.c");
const MANIFEST: &str = ".better-fonts.json";
pub(crate) fn binary(bundle: &Path) -> Result<PathBuf> {
    let directory = bundle.join("Contents/MacOS");
    if let Ok(plist) = plist::Value::from_file(bundle.join("Contents/Info.plist"))
        && let Some(name) = plist
            .as_dictionary()
            .and_then(|d| d.get("CFBundleExecutable"))
            .and_then(plist::Value::as_string)
    {
        if name.is_empty()
            || Path::new(name).components().count() != 1
            || name == ".."
            || name.contains(['/', '\\'])
        {
            return Err(invalid("invalid CFBundleExecutable"));
        }
        let path = directory.join(name);
        if !fs::symlink_metadata(&path)?.is_file() {
            return Err(invalid("native executable must be a regular file"));
        }
        return Ok(path);
    }
    let mut candidates = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if entry.file_type()?.is_file()
            && !name.starts_with('.')
            && ![".orig", ".bak", ".dylib"]
                .iter()
                .any(|suffix| name.ends_with(suffix))
        {
            candidates.push(entry.path());
        }
    }
    if candidates.len() != 1 {
        return Err(invalid(
            "native executable is ambiguous; supply a valid CFBundleExecutable plist",
        ));
    }
    Ok(candidates.remove(0))
}
fn suffix(path: &Path, extension: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(extension);
    value.into()
}
fn validate_paths(bundle: &Path, executable: &Path) -> Result<()> {
    let directory = executable.parent().unwrap();
    if !fs::canonicalize(directory)?.starts_with(fs::canonicalize(bundle)?) {
        return Err(invalid("native executable directory escapes bundle"));
    }
    for path in [
        suffix(executable, ".orig"),
        suffix(executable, ".bak"),
        directory.join("libfonthook.dylib"),
        directory.join(MANIFEST),
    ] {
        match fs::symlink_metadata(path) {
            Ok(m) if !m.is_file() => {
                return Err(invalid("native hook artifacts must be regular files"));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
pub(crate) fn status(bundle: &Path) -> Result<(bool, Option<String>)> {
    let executable = binary(bundle)?;
    validate_paths(bundle, &executable)?;
    let directory = executable.parent().unwrap();
    let original = suffix(&executable, ".orig");
    let library = directory.join("libfonthook.dylib");
    let patched = original.is_file() && library.is_file();
    let font = if patched {
        match fs::read(directory.join(MANIFEST)) {
            Ok(data) => Some(serde_json::from_slice::<String>(&data)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        }
    } else {
        None
    };
    Ok((patched, font))
}
fn c_string(font: &str) -> String {
    format!(
        "\"{}\"",
        font.as_bytes()
            .iter()
            .map(|b| format!("\\{b:03o}"))
            .collect::<String>()
    )
}
fn compile(
    executable: &Path,
    font: &str,
    cache: &Path,
    runner: &dyn Runner,
    token: &CancellationToken,
) -> Result<(PathBuf, PathBuf)> {
    let arch_output = run(
        runner,
        "lipo",
        &["-archs".into(), executable.as_os_str().into()],
        token,
    )?;
    let text = std::str::from_utf8(&arch_output)
        .map_err(|_| invalid("invalid lipo architecture output"))?;
    let architectures: Vec<_> = text.split_whitespace().collect();
    if architectures.is_empty()
        || architectures
            .iter()
            .any(|a| !["arm64", "arm64e", "x86_64", "i386"].contains(a))
    {
        return Err(invalid("unsupported application architecture"));
    }
    let key = format!(
        "{:x}",
        Sha256::digest(format!("{font}\n{text}\n{HOOK}\n{LAUNCHER}"))
    );
    let build = cache.join("build");
    fs::create_dir_all(&build)?;
    let output = build.join(key);
    let library = output.join("libfonthook.dylib");
    let wrapper = output.join("launcher");
    if library.is_file() && wrapper.is_file() {
        return Ok((library, wrapper));
    }
    let work = tempfile::Builder::new()
        .prefix("compile-")
        .tempdir_in(&build)?;
    let source = work.path().join("hook.m");
    let launcher = work.path().join("launcher.c");
    fs::write(&source, HOOK)?;
    fs::write(&launcher, LAUNCHER)?;
    let mut arch = Vec::<OsString>::new();
    for value in architectures {
        arch.extend(["-arch".into(), value.into()]);
    }
    let temp_library = work.path().join("libfonthook.dylib");
    let temp_wrapper = work.path().join("launcher");
    let mut args: Vec<OsString> = vec![
        "-dynamiclib".into(),
        "-framework".into(),
        "CoreText".into(),
        "-framework".into(),
        "Foundation".into(),
        format!("-DTARGET_FONT_NAME={}", c_string(font)).into(),
    ];
    args.extend(arch.clone());
    args.extend(["-o".into(), temp_library.as_os_str().into(), source.into()]);
    run(runner, "clang", &args, token)?;
    let mut args = vec!["-O3".into()];
    args.extend(arch);
    args.extend([
        "-o".into(),
        temp_wrapper.as_os_str().into(),
        launcher.into(),
    ]);
    run(runner, "clang", &args, token)?;
    for path in [&temp_library, &temp_wrapper] {
        if fs::metadata(path)?.len() == 0 {
            return Err(invalid("compiler produced an empty artifact"));
        }
    }
    match fs::rename(work.path(), &output) {
        Ok(()) => {}
        Err(e) if output.is_dir() => {
            if !library.is_file() || !wrapper.is_file() {
                return Err(e.into());
            }
        }
        Err(e) => return Err(e.into()),
    }
    Ok((library, wrapper))
}
pub(crate) fn transform(
    bundle: &Path,
    font: &str,
    patch: bool,
    cache: &Path,
    runner: &dyn Runner,
    token: &CancellationToken,
) -> Result<()> {
    let executable = binary(bundle)?;
    validate_paths(bundle, &executable)?;
    let directory = executable.parent().unwrap();
    let original = suffix(&executable, ".orig");
    let backup = suffix(&executable, ".bak");
    let library = directory.join("libfonthook.dylib");
    if patch {
        if font.trim().is_empty() || font.contains('\0') {
            return Err(invalid("invalid font name"));
        }
        let source = if backup.is_file() {
            &backup
        } else if original.is_file() {
            &original
        } else {
            &executable
        };
        let (built_library, wrapper) = compile(source, font, cache, runner, token)?;
        if !backup.exists() {
            fs::copy(source, &backup)?;
        }
        fs::copy(&backup, &original)?;
        fs::copy(built_library, &library)?;
        fs::copy(wrapper, &executable)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))?;
            fs::set_permissions(&original, fs::Permissions::from_mode(0o755))?;
        }
        fs::write(directory.join(MANIFEST), serde_json::to_vec(font)?)?;
    } else {
        let source = if backup.is_file() { &backup } else { &original };
        fs::copy(source, &executable)?;
        for path in [&original, &library, &directory.join(MANIFEST)] {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
    }
    Ok(())
}
