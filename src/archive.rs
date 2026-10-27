//! Chromium Pickle header framing verified against electron/asar disk.ts.
//! Unknown JSON properties survive edits; unchanged packed data is streamed.
use crate::{Result, invalid};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

pub struct Archive {
    path: PathBuf,
    file: File,
    header: Value,
    start: u64,
    entries: BTreeMap<String, (u64, u64)>,
}

fn collect(
    node: &Value,
    prefix: &str,
    entries: &mut BTreeMap<String, (u64, u64)>,
    available: u64,
) -> Result<()> {
    let files = node
        .get("files")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("ASAR directory requires files object"))?;
    for (name, child) in files {
        if name.is_empty() || [".", ".."].contains(&name.as_str()) || name.contains(['/', '\\']) {
            return Err(invalid("invalid ASAR entry name"));
        }
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        if child.get("files").is_some() {
            collect(child, &path, entries, available)?;
        } else if child.get("link").is_some() {
            if !child["link"].as_str().is_some_and(|v| !v.is_empty()) {
                return Err(invalid("invalid ASAR link"));
            }
        } else {
            let size = child["size"]
                .as_u64()
                .ok_or_else(|| invalid("invalid ASAR file size"))?;
            if child
                .get("unpacked")
                .is_some_and(|v| v != &Value::Bool(false))
            {
                if child["unpacked"] != Value::Bool(true) {
                    return Err(invalid("invalid unpacked flag"));
                }
                continue;
            }
            let offset = child["offset"]
                .as_str()
                .filter(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|v| v.parse::<u64>().ok())
                .ok_or_else(|| invalid("invalid ASAR file offset"))?;
            if offset.checked_add(size).is_none_or(|end| end > available) {
                return Err(invalid("ASAR file exceeds archive payload"));
            }
            entries.insert(path, (offset, size));
        }
    }
    Ok(())
}
fn entry_mut<'a>(header: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    let mut node = header;
    for name in path.split('/') {
        node = node.get_mut("files")?.get_mut(name)?;
    }
    Some(node)
}
fn hex(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

impl Archive {
    pub fn open(path: &Path) -> Result<Self> {
        let mut file = File::open(path)?;
        let length = file.metadata()?.len();
        let mut bytes = [0u8; 16];
        file.read_exact(&mut bytes)?;
        let word = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
        let payload = word(4) as u64;
        let json_length = word(12) as u64;
        let start = 8 + payload;
        if word(0) != 4
            || payload < 8
            || word(8) as u64 != payload - 4
            || json_length > payload - 8
            || start > length
            || !payload.is_multiple_of(4)
            || json_length > 64 * 1024 * 1024
        {
            return Err(invalid(
                "invalid ASAR Pickle header (maximum JSON header: 64 MiB)",
            ));
        }
        let mut json = vec![0; json_length as usize];
        file.read_exact(&mut json)?;
        let header = serde_json::from_slice(&json)?;
        let mut entries = BTreeMap::new();
        collect(&header, "", &mut entries, length - start)?;
        Ok(Self {
            path: path.into(),
            file,
            header,
            start,
            entries,
        })
    }
    pub fn header(&self) -> &Value {
        &self.header
    }
    pub fn files(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
    pub fn extract(&mut self, path: &str) -> Result<Vec<u8>> {
        let &(offset, size) = self
            .entries
            .get(path)
            .ok_or_else(|| invalid(format!("packed ASAR file not found: {path}")))?;
        let length =
            usize::try_from(size).map_err(|_| invalid("ASAR file too large for this platform"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|e| invalid(e.to_string()))?;
        bytes.resize(length, 0);
        self.file.seek(SeekFrom::Start(self.start + offset))?;
        self.file.read_exact(&mut bytes)?;
        Ok(bytes)
    }
    /// Publish a replacement only after the complete rebuilt archive is synced.
    pub fn replace(&mut self, path: &str, data: &[u8]) -> Result<()> {
        if !self.entries.contains_key(path) {
            return Err(invalid(format!("packed ASAR file not found: {path}")));
        }
        let mut header = self.header.clone();
        let target = entry_mut(&mut header, path).unwrap();
        if let Some(integrity) = target.get_mut("integrity") {
            if integrity["algorithm"] != "SHA256" {
                return Err(invalid("unsupported ASAR integrity algorithm"));
            }
            let block = integrity["blockSize"]
                .as_u64()
                .filter(|v| *v > 0)
                .and_then(|v| usize::try_from(v).ok())
                .ok_or_else(|| invalid("invalid ASAR integrity blockSize"))?;
            integrity["hash"] = json!(hex(data));
            integrity["blocks"] = json!(if data.is_empty() {
                vec![hex(data)]
            } else {
                data.chunks(block).map(hex).collect::<Vec<_>>()
            });
        }
        let mut offset = 0u64;
        for (name, &(_, size)) in &self.entries {
            let size = if name == path {
                data.len() as u64
            } else {
                size
            };
            let entry = entry_mut(&mut header, name).unwrap();
            entry["size"] = json!(size);
            entry["offset"] = json!(offset.to_string());
            offset = offset
                .checked_add(size)
                .ok_or_else(|| invalid("ASAR payload overflow"))?;
        }
        let json = serde_json::to_vec(&header)?;
        let padded = (json.len() + 3) & !3;
        let payload =
            u32::try_from(padded + 8).map_err(|_| invalid("ASAR header exceeds Pickle range"))?;
        let mut output = tempfile::NamedTempFile::new_in(crate::parent(&self.path))?;
        output
            .as_file()
            .set_permissions(self.file.metadata()?.permissions())?;
        for attribute in xattr::list(&self.path)? {
            if let Some(value) = xattr::get(&self.path, &attribute)? {
                xattr::set(output.path(), &attribute, &value)?;
            }
        }
        for word in [4, payload, payload - 4, json.len() as u32] {
            output.write_all(&word.to_le_bytes())?;
        }
        output.write_all(&json)?;
        output.write_all(&[0; 3][..padded - json.len()])?;
        for (name, &(old_offset, size)) in &self.entries {
            if name == path {
                output.write_all(data)?;
            } else {
                self.file.seek(SeekFrom::Start(self.start + old_offset))?;
                let copied = std::io::copy(&mut (&mut self.file).take(size), &mut output)?;
                if copied != size {
                    return Err(invalid("ASAR payload truncated during replacement"));
                }
            }
        }
        output.as_file().sync_all()?;
        output.persist(&self.path).map_err(|e| e.error)?;
        *self = Self::open(&self.path)?;
        Ok(())
    }
}
