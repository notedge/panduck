use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use panduck_types::{AdapterError, Result};

/// Atomically publish UTF-8 text to `path`, replacing an existing file when present.
pub fn publish_text(path: impl AsRef<Path>, content: &str) -> Result<()> {
    publish_bytes(path, content.as_bytes())
}

/// Atomically publish raw bytes to `path`, replacing an existing file when present.
pub fn publish_bytes(path: impl AsRef<Path>, content: &[u8]) -> Result<()> {
    let path = path.as_ref();
    publish_bytes_atomic(path, content)
        .map_err(|error| AdapterError::io(error, Some(path.display().to_string())))
}

fn publish_bytes_atomic(path: &Path, content: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "destination has no parent directory")
        })?;
    if !parent.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "destination parent directory does not exist",
        ));
    }

    let staging_path = staging_path(parent, path)?;
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging_path)?;
        file.write_all(content)?;
        file.sync_all()?;
    }

    replace_path(&staging_path, path)?;
    sync_parent(path)?;
    Ok(())
}

fn staging_path(parent: &Path, target: &Path) -> io::Result<PathBuf> {
    let file_name = target
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "destination has no file name"))?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    Ok(parent.join(format!(
        ".{}.{}.{}.tmp",
        file_name.to_string_lossy(),
        std::process::id(),
        nanos
    )))
}

fn replace_path(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            fs::remove_file(to)?;
            fs::rename(from, to)
        }
        Err(error) => Err(error),
    }
}

fn sync_parent(path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "destination has no parent directory")
        })?;
    if let Ok(dir) = File::open(parent) {
        let _ = dir.sync_all();
    }
    Ok(())
}
