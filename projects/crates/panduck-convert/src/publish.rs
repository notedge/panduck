use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use notedown_formats::export::markdown_project::MarkdownProject;
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

/// Published Markdown project layout on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedMarkdownProject {
    /// Output directory root.
    pub output_dir: PathBuf,
    /// Written `index.md` path.
    pub index_path: PathBuf,
    /// Written asset paths relative to `output_dir`.
    pub asset_paths: Vec<String>,
    /// Written `panduck.report.json` path.
    pub report_path: PathBuf,
}

/// Writes `index.md`, `assets/*`, and `panduck.report.json` under `output_dir`.
pub fn publish_markdown_project(
    output_dir: impl AsRef<Path>,
    project: &MarkdownProject,
    report_json: &str,
) -> Result<PublishedMarkdownProject> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)
        .map_err(|error| AdapterError::io(error, Some(output_dir.display().to_string())))?;
    let assets_dir = output_dir.join("assets");
    fs::create_dir_all(&assets_dir)
        .map_err(|error| AdapterError::io(error, Some(assets_dir.display().to_string())))?;

    let index_path = output_dir.join("index.md");
    publish_text(&index_path, &project.index_markdown)?;

    let mut asset_paths = Vec::with_capacity(project.assets.len());
    for asset in &project.assets {
        let file_name = asset
            .relative_path
            .strip_prefix("assets/")
            .ok_or_else(|| AdapterError::invalid_input(format!("asset path must start with assets/: {}", asset.relative_path)))?;
        let path = assets_dir.join(file_name);
        publish_bytes(&path, &asset.bytes)?;
        asset_paths.push(asset.relative_path.clone());
    }

    let report_path = output_dir.join("panduck.report.json");
    publish_text(&report_path, report_json)?;

    Ok(PublishedMarkdownProject {
        output_dir: output_dir.to_path_buf(),
        index_path,
        asset_paths,
        report_path,
    })
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
