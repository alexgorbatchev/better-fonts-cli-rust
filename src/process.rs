use crate::{Error, Result};
use std::{
    ffi::{OsStr, OsString},
    fs,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::Duration,
};
use tokio_util::sync::CancellationToken;
pub trait Runner {
    fn output(
        &self,
        program: &OsStr,
        args: &[OsString],
        token: &CancellationToken,
    ) -> Result<Vec<u8>>;
}
pub struct SystemRunner {
    pub scratch: PathBuf,
}
impl Default for SystemRunner {
    fn default() -> Self {
        Self {
            scratch: PathBuf::from(".tmp/process"),
        }
    }
}
impl Runner for SystemRunner {
    fn output(
        &self,
        program: &OsStr,
        args: &[OsString],
        token: &CancellationToken,
    ) -> Result<Vec<u8>> {
        check(token)?;
        fs::create_dir_all(&self.scratch)?;
        let mut stdout = tempfile::NamedTempFile::new_in(&self.scratch)?;
        let mut stderr = tempfile::NamedTempFile::new_in(&self.scratch)?;
        let mut command = Command::new(program);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command
            .args(args)
            .stdin(Stdio::null())
            .stdout(stdout.as_file().try_clone()?)
            .stderr(stderr.as_file().try_clone()?)
            .spawn()?;
        let status = loop {
            if token.is_cancelled() {
                #[cfg(unix)]
                // SAFETY: the child leads its own process group; negative PID targets only that group.
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::Cancelled);
            }
            if let Some(status) = child.try_wait()? {
                break status;
            }
            thread::sleep(Duration::from_millis(10));
        };
        if !status.success() {
            stderr.seek(SeekFrom::Start(0))?;
            let mut message = String::new();
            stderr.read_to_string(&mut message)?;
            return Err(Error::Process {
                program: program.to_string_lossy().into_owned(),
                message: format!("{status}: {}", message.trim()),
            });
        }
        stdout.seek(SeekFrom::Start(0))?;
        let mut result = Vec::new();
        stdout.read_to_end(&mut result)?;
        Ok(result)
    }
}
pub(crate) fn check(token: &CancellationToken) -> Result<()> {
    if token.is_cancelled() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
pub(crate) fn run(
    runner: &dyn Runner,
    program: &str,
    args: &[OsString],
    token: &CancellationToken,
) -> Result<Vec<u8>> {
    runner.output(OsStr::new(program), args, token)
}
