//! Electron fuse wire v1. Validate every architecture before changing any byte.
use crate::{Result, invalid};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
pub const SENTINEL: &[u8] = b"dL7pKGdnNz796PbbjQWNKmHXBZaB9tsX";
fn positions(file: &mut File) -> Result<Vec<u64>> {
    file.seek(SeekFrom::Start(0))?;
    let mut tail = Vec::new();
    let mut offset = 0u64;
    let mut found = Vec::new();
    loop {
        let mut chunk = [0; 65536];
        let count = file.read(&mut chunk)?;
        if count == 0 {
            break;
        }
        let retained = tail.len();
        tail.extend_from_slice(&chunk[..count]);
        for (i, bytes) in tail.windows(SENTINEL.len()).enumerate() {
            if bytes == SENTINEL {
                let position = offset - retained as u64 + i as u64;
                if found.last() != Some(&position) {
                    found.push(position);
                }
            }
        }
        offset += count as u64;
        let keep = tail.len().saturating_sub(SENTINEL.len() - 1);
        tail.drain(..keep);
    }
    if found.is_empty() || found.len() > 2 {
        return Err(invalid("expected one or two Electron fuse sentinels"));
    }
    Ok(found)
}
fn states(file: &mut File) -> Result<Vec<(u64, u8)>> {
    let mut result = Vec::new();
    let length = file.metadata()?.len();
    for position in positions(file)? {
        let start = position + SENTINEL.len() as u64;
        file.seek(SeekFrom::Start(start))?;
        let mut header = [0; 2];
        file.read_exact(&mut header)?;
        if header[0] != 1 || header[1] < 5 || start + 2 + header[1] as u64 > length {
            return Err(invalid("unsupported or truncated Electron fuse wire"));
        }
        file.seek(SeekFrom::Start(start + 6))?;
        let mut state = [0];
        file.read_exact(&mut state)?;
        if !b"01".contains(&state[0]) {
            return Err(invalid("integrity fuse is removed or invalid"));
        }
        result.push((start + 6, state[0]));
    }
    Ok(result)
}
pub fn disable(path: &Path) -> Result<()> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    let updates = states(&mut file)?;
    for (position, state) in updates {
        if state != b'0' {
            file.seek(SeekFrom::Start(position))?;
            file.write_all(b"0")?;
        }
    }
    file.sync_all()?;
    Ok(())
}
pub fn is_disabled(path: &Path) -> Result<bool> {
    Ok(states(&mut File::open(path)?)?
        .iter()
        .all(|(_, state)| *state == b'0'))
}
pub fn framework_binary(bundle: &Path) -> Result<PathBuf> {
    let frameworks = bundle.join("Contents/Frameworks");
    let mut candidates = vec![frameworks.join("Electron Framework.framework")];
    let mut others = std::fs::read_dir(&frameworks)?.collect::<std::io::Result<Vec<_>>>()?;
    others.sort_by_key(|e| e.file_name());
    for entry in others {
        if entry.path().extension().is_some_and(|v| v == "framework")
            && !candidates.contains(&entry.path())
        {
            candidates.push(entry.path());
        }
    }
    let mut fallback = None;
    for framework in candidates {
        let Some(name) = framework.file_stem() else {
            continue;
        };
        for relative in [
            PathBuf::from(name),
            PathBuf::from("Versions/A").join(name),
            PathBuf::from("Versions/Current").join(name),
        ] {
            let path = framework.join(relative);
            if path.is_file() {
                if fallback.is_none() {
                    fallback = Some(path.clone());
                }
                if positions(&mut File::open(&path)?).is_ok() {
                    return Ok(path);
                }
            }
        }
    }
    fallback.ok_or_else(|| invalid("Electron framework binary not found"))
}
