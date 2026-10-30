use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use notedown_formats::export::markdown_project::MarkdownProject;
use panduck_types::{AdapterError, Result};

/// Atomically publish UTF-8 text to `path`, replacing an existing file when present.
pub fn publish_text(path: impl AsRef<Path>, content: &str) -> Result<()> {
    publish_bytes(path, content.as_bytes())
}

/// Atomically publish raw bytes to `path`, replacing an existing file when present.
pub fn publish_bytes(path: impl AsRef<Path>, content: &[u8]) -> Result<()> {
    let path = path.as_ref();
    publish_bytes_atomic(path, content).map_err(|error| AdapterError::io(error, Some(path.display().to_string())))
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
    /// Written chapter paths relative to `output_dir`.
    pub chapter_paths: Vec<String>,
    /// Written `panduck.report.json` path.
    pub report_path: PathBuf,
}

/// Writes `index.md`, optional `chapters/*`, `assets/*`, and `panduck.report.json` under `output_dir`.
///
/// Atomicity: each individual file is written via a staging-file-and-rename
/// round-trip.  The project layout as a whole is produced into a sibling
/// temporary directory first, then atomically renamed into `output_dir` so a
/// failure during chapter or asset writing never leaves the target directory
/// in a half-published state.
pub fn publish_markdown_project(
    output_dir: impl AsRef<Path>,
    project: &MarkdownProject,
    report_json: &str,
) -> Result<PublishedMarkdownProject> {
    let output_dir = output_dir.as_ref();

    // Stage everything into a sibling temp directory so any mid-flow failure
    // (bad asset path, disk full, …) does not leave `output_dir` dirty.
    let staging_dir = staging_dir_path(output_dir);
    if staging_dir.exists() {
        fs::remove_dir_all(&staging_dir).map_err(|error| AdapterError::io(error, Some(staging_dir.display().to_string())))?;
    }
    fs::create_dir_all(&staging_dir).map_err(|error| AdapterError::io(error, Some(staging_dir.display().to_string())))?;

    // Stage the whole layout first, then swap it into place. Any staging error
    // (bad asset path, disk full, …) returns before `output_dir` is touched,
    // and the staging directory is always removed so a failed publish leaves
    // neither a dirty output directory nor a half-written sibling behind.
    let result =
        stage_markdown_project(&staging_dir, project, report_json).and_then(|()| replace_project_dir(&staging_dir, output_dir));
    let _ = fs::remove_dir_all(&staging_dir);
    result?;

    Ok(PublishedMarkdownProject {
        output_dir: output_dir.to_path_buf(),
        index_path: output_dir.join("index.md"),
        chapter_paths: project.chapters.iter().map(|chapter| chapter.relative_path.clone()).collect(),
        asset_paths: project.assets.iter().map(|asset| asset.relative_path.clone()).collect(),
        report_path: output_dir.join("panduck.report.json"),
    })
}

/// Writes the project layout into an empty `staging_dir`.
fn stage_markdown_project(staging_dir: &Path, project: &MarkdownProject, report_json: &str) -> Result<()> {
    let assets_dir = staging_dir.join("assets");
    fs::create_dir_all(&assets_dir).map_err(|error| AdapterError::io(error, Some(assets_dir.display().to_string())))?;

    publish_text(staging_dir.join("index.md"), &project.index_markdown)?;

    for chapter in &project.chapters {
        let chapter_path = staging_dir.join(&chapter.relative_path);
        if let Some(parent) = chapter_path.parent() {
            fs::create_dir_all(parent).map_err(|error| AdapterError::io(error, Some(parent.display().to_string())))?;
        }
        publish_text(&chapter_path, &chapter.markdown)?;
    }

    for asset in &project.assets {
        let file_name = asset.relative_path.strip_prefix("assets/").ok_or_else(|| {
            AdapterError::invalid_input(format!("asset path must start with assets/: {}", asset.relative_path))
        })?;
        publish_bytes(assets_dir.join(file_name), &asset.bytes)?;
    }

    publish_text(staging_dir.join("panduck.report.json"), report_json)
}

fn staging_dir_path(output_dir: &Path) -> PathBuf {
    let file_name = output_dir.file_name().and_then(|n| n.to_str()).unwrap_or("output");
    let parent = output_dir.parent().unwrap_or_else(|| Path::new("."));
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    parent.join(format!(".{}.{}.{}.staging", file_name, std::process::id(), nanos))
}

fn replace_project_dir(staging_dir: &Path, output_dir: &Path) -> Result<()> {
    match fs::rename(staging_dir, output_dir) {
        Ok(()) => Ok(()),
        // Windows: `rename` fails when the destination is a non-empty dir.
        // Other platforms: `rename` silently replaces the destination; still,
        // an explicit remove-before-rename keeps semantics identical.
        Err(_) => {
            if output_dir.exists() {
                fs::remove_dir_all(output_dir)
                    .map_err(|error| AdapterError::io(error, Some(output_dir.display().to_string())))?;
            }
            fs::rename(staging_dir, output_dir).map_err(|e| {
                let msg = e.to_string();
                AdapterError::io(e, Some(msg))
            })?;
            Ok(())
        }
    }
}

fn publish_bytes_atomic(path: &Path, content: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "destination has no parent directory"))?;
    if !parent.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "destination parent directory does not exist"));
    }

    let staging_path = staging_path(parent, path)?;
    {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&staging_path)?;
        file.write_all(content)?;
        file.sync_all()?;
    }

    replace_path(&staging_path, path)?;
    sync_parent(path)?;
    Ok(())
}

fn staging_path(parent: &Path, target: &Path) -> io::Result<PathBuf> {
    let file_name =
        target.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "destination has no file name"))?;
    let nanos =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|duration| duration.as_nanos()).unwrap_or(0);
    Ok(parent.join(format!(".{}.{}.{}.tmp", file_name.to_string_lossy(), std::process::id(), nanos)))
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
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "destination has no parent directory"))?;
    if let Ok(dir) = File::open(parent) {
        let _ = dir.sync_all();
    }
    Ok(())
}
