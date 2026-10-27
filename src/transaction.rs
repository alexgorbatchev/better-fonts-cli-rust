use crate::{Error, Result, invalid, process::check};
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
};
use tokio_util::sync::CancellationToken;
pub(crate) fn copy_tree(
    source: &Path,
    destination: &Path,
    token: &CancellationToken,
) -> Result<()> {
    check(token)?;
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        #[cfg(unix)]
        std::os::unix::fs::symlink(fs::read_link(source)?, destination)?;
        #[cfg(not(unix))]
        return Err(invalid("application symlinks require Unix"));
    } else if metadata.is_dir() {
        fs::create_dir(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_tree(&entry.path(), &destination.join(entry.file_name()), token)?;
        }
        fs::set_permissions(destination, metadata.permissions())?;
    } else if metadata.is_file() {
        fs::copy(source, destination)?;
    } else {
        return Err(invalid(
            "application contains unsupported special filesystem object",
        ));
    }
    for attribute in xattr::list(source)? {
        if let Some(value) = xattr::get(source, &attribute)? {
            xattr::set(destination, &attribute, &value)?;
        }
    }
    Ok(())
}
pub(crate) fn lock(path: &Path) -> Result<File> {
    use sha2::{Digest, Sha256};
    let key = format!("{:x}", Sha256::digest(path.as_os_str().as_encoded_bytes()));
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    // Keep the lock inode beside the bundle across publication and cache changes.
    // Removing it after unlocking would let a waiter lock an obsolete inode.
    let file = options.open(crate::parent(path).join(format!(".better-fonts-{key}.lock")))?;
    file.try_lock()
        .map_err(|e| invalid(format!("application is busy or cannot be locked: {e}")))?;
    Ok(file)
}
pub(crate) fn replace(original: &Path, staged: &Path, token: &CancellationToken) -> Result<()> {
    check(token)?;
    let backup = tempfile::Builder::new()
        .prefix(".better-fonts-backup-")
        .tempdir_in(
            original
                .parent()
                .ok_or_else(|| invalid("app has no parent directory"))?,
        )?;
    let preserved = backup.path().join("original.app");
    fs::rename(original, &preserved)?;
    if let Err(error) = fs::rename(staged, original) {
        if let Err(restore) = fs::rename(&preserved, original) {
            let saved = backup.keep();
            return Err(invalid(format!(
                "publication failed: {error}; rollback failed: {restore}; original preserved at {}",
                saved.display()
            )));
        }
        return Err(error.into());
    }
    backup
        .close()
        .map_err(|e| Error::Committed(format!("backup cleanup failed: {e}")))?;
    Ok(())
}
