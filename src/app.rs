use crate::{
    Error, Result,
    archive::Archive,
    injection, invalid, native,
    process::{Runner, check, run},
    transaction,
};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tokio_util::sync::CancellationToken;
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Driver {
    Electron,
    NativeHook,
}
impl std::str::FromStr for Driver {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "electron" => Ok(Self::Electron),
            "native-hook" => Ok(Self::NativeHook),
            _ => Err(invalid(format!("unknown driver: {s}"))),
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct App {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub process_name: String,
    pub driver: Driver,
    pub asar_path: PathBuf,
    pub preload_path: String,
}
#[derive(Debug, Serialize)]
pub struct Status {
    pub installed: bool,
    pub patched: bool,
    pub font: Option<String>,
}
pub struct Apply<'a> {
    pub font: &'a str,
    pub patch: bool,
    pub restart: bool,
    pub dry_run: bool,
    pub cache: &'a Path,
}
fn safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}
impl App {
    fn archive_path(&self, bundle: &Path) -> Result<PathBuf> {
        if !safe_relative(&self.asar_path) {
            return Err(invalid(
                "ASAR path must be relative to Contents/Resources without parent traversal",
            ));
        }
        let path = bundle.join("Contents/Resources").join(&self.asar_path);
        let actual = fs::canonicalize(&path)?;
        if !actual.starts_with(fs::canonicalize(bundle)?) {
            return Err(invalid("ASAR path escapes application bundle"));
        }
        Ok(actual)
    }
    pub fn status(&self) -> Result<Status> {
        match fs::metadata(&self.path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Status {
                    installed: false,
                    patched: false,
                    font: None,
                });
            }
            Err(e) => return Err(e.into()),
            Ok(m) if !m.is_dir() => return Err(invalid("application is not a directory")),
            _ => {}
        }
        let (patched, font) = match self.driver {
            Driver::NativeHook => native::status(&self.path)?,
            Driver::Electron => {
                let mut archive = Archive::open(&self.archive_path(&self.path)?)?;
                let data = archive.extract(&self.preload_path)?;
                let text = std::str::from_utf8(&data)
                    .map_err(|_| invalid("preload is not UTF-8 JavaScript"))?;
                let font = injection::detect(text, &self.id)?;
                (font.is_some(), font)
            }
        };
        Ok(Status {
            installed: true,
            patched,
            font,
        })
    }
    pub fn apply(
        &self,
        options: &Apply<'_>,
        runner: &dyn Runner,
        token: &CancellationToken,
    ) -> Result<()> {
        check(token)?;
        let status = self.status()?;
        if !status.installed {
            return Err(invalid(format!(
                "application is not installed: {}",
                self.path.display()
            )));
        }
        if !options.patch && !status.patched {
            return Ok(());
        }
        if options.dry_run {
            return Ok(());
        }
        let original = fs::canonicalize(&self.path)?;
        let _lock = transaction::lock(&original)?;
        let stage = tempfile::Builder::new()
            .prefix(".better-fonts-stage-")
            .tempdir_in(original.parent().unwrap())?;
        let staged = stage.path().join(original.file_name().unwrap());
        transaction::copy_tree(&original, &staged, token)?;
        match self.driver {
            Driver::NativeHook => native::transform(
                &staged,
                options.font,
                options.patch,
                options.cache,
                runner,
                token,
            )?,
            Driver::Electron => {
                let mut archive = Archive::open(&self.archive_path(&staged)?)?;
                let data = archive.extract(&self.preload_path)?;
                let text = std::str::from_utf8(&data)
                    .map_err(|_| invalid("preload is not UTF-8 JavaScript"))?;
                let text = if options.patch {
                    injection::inject(text, &self.id, options.font)?
                } else {
                    injection::strip(text, &self.id)?
                };
                archive.replace(&self.preload_path, text.as_bytes())?;
                crate::fuses::disable(&crate::fuses::framework_binary(&staged)?)?;
            }
        }
        run(
            runner,
            "codesign",
            &[
                "--force".into(),
                "--deep".into(),
                "--sign".into(),
                "-".into(),
                staged.as_os_str().into(),
            ],
            token,
        )?;
        if options.restart && !self.process_name.is_empty() {
            run(
                runner,
                "osascript",
                &[
                    "-e".into(),
                    "on run argv\n tell application (item 1 of argv) to quit\nend run".into(),
                    self.process_name.clone().into(),
                ],
                token,
            )?;
        }
        transaction::replace(&original, &staged, token)?;
        if options.restart && !self.process_name.is_empty() {
            run(
                runner,
                "open",
                &["-a".into(), self.process_name.clone().into()],
                token,
            )
            .map_err(|e| Error::Committed(format!("application restart failed: {e}")))?;
        }
        Ok(())
    }
}
mod registry;
pub use registry::{registry, select};
