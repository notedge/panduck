use std::path::Path;

use fs_tools::{publish_bytes_with_options, OverwritePolicy, PublishOptions};
use panduck_types::{AdapterError, Result};

/// Atomically publish UTF-8 text to `path`, replacing an existing file when present.
pub fn publish_text(path: impl AsRef<Path>, content: &str) -> Result<()> {
    publish_bytes(path, content.as_bytes())
}

/// Atomically publish raw bytes to `path`, replacing an existing file when present.
pub fn publish_bytes(path: impl AsRef<Path>, content: &[u8]) -> Result<()> {
    let path = path.as_ref();
    publish_bytes_with_options(
        path,
        content,
        PublishOptions {
            overwrite: OverwritePolicy::Replace,
            fsync_file: true,
            fsync_parent: true,
        },
    )
    .map_err(|error| AdapterError::io(error, Some(path.display().to_string())))
}
