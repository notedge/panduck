use notedown_formats::{FormatError, InspectBudget, inspect_docx_bytes};
use panduck_types::{AdapterError, Result};

use crate::format_error;

/// Container-level budgets forwarded from the CLI.
///
/// Only keys that this build can enforce are modelled here. Keys such as
/// `layout_nodes` or `wall_ms` have no enforcement point in the current
/// pipeline and are rejected by the CLI instead of silently ignored.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConversionBudgets {
    /// Maximum total uncompressed bytes declared by a container.
    pub decompressed_bytes: Option<u64>,
    /// Maximum member count indexed from one container.
    pub container_entries: Option<u64>,
}

impl ConversionBudgets {
    /// Parses the CLI budget object. Unknown keys are ignored here because the
    /// CLI rejects them before invoking the native layer.
    pub fn from_json(raw: Option<&str>) -> Self {
        let Some(raw) = raw else {
            return Self::default();
        };
        let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(raw) else {
            return Self::default();
        };
        let read = |key: &str| map.get(key).and_then(serde_json::Value::as_u64);
        Self { decompressed_bytes: read("decompressed_bytes"), container_entries: read("container_entries") }
    }

    /// Returns whether any container budget needs enforcement.
    pub fn is_empty(&self) -> bool {
        self.decompressed_bytes.is_none() && self.container_entries.is_none()
    }

    /// Rejects a container before import when it exceeds a declared budget.
    ///
    /// Enforcement reads container metadata only. ZIP-based DOCX and EPUB
    /// containers are inspected here. PDF and OLE containers have no public
    /// metadata surface, so they rely on the format layer's own decode limits.
    pub fn enforce_container(&self, from: &str, label: &str, bytes: &[u8]) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }
        match from {
            "docx" => self.enforce_docx(label, bytes),
            "epub" => self.enforce_epub(label, bytes),
            _ => Ok(()),
        }
    }

    fn enforce_docx(&self, label: &str, bytes: &[u8]) -> Result<()> {
        let default = InspectBudget::default();
        let budget = InspectBudget {
            max_decoded_bytes: u64::MAX,
            max_container_members: self.container_entries.unwrap_or(default.max_container_members),
        };
        let inspect = inspect_docx_bytes(label, bytes, None, false, budget).map_err(format_error::map_format_error)?;
        self.check_declared("docx", inspect.parts.iter().map(|part| part.uncompressed_size))
    }

    fn enforce_epub(&self, label: &str, bytes: &[u8]) -> Result<()> {
        let package = acorn_epub::OcfPackage::open(label.to_string(), bytes.to_vec())
            .map_err(|error| format_error::map_format_error(FormatError::parse("epub", error.to_string())))?;
        let members = package.member_paths();
        if let Some(limit) = self.container_entries {
            if members.len() as u64 > limit {
                return format_error::fail(AdapterError::diagnostic(notedown_formats::from_format_error(&FormatError::budget(
                    "epub",
                    format!("container_entries budget exhausted: indexed {} exceeds limit {limit}", members.len()),
                ))));
            }
        }
        self.check_declared(
            "epub",
            members.iter().filter_map(|path| package.member(path)).map(|member| member.uncompressed_size),
        )
    }

    fn check_declared(&self, format: &str, sizes: impl Iterator<Item = u64>) -> Result<()> {
        let Some(limit) = self.decompressed_bytes else {
            return Ok(());
        };
        let declared = sizes.fold(0u64, u64::saturating_add);
        if declared > limit {
            return format_error::fail(AdapterError::diagnostic(notedown_formats::from_format_error(&FormatError::budget(
                format,
                format!("decompressed_bytes budget exhausted: declared {declared} exceeds limit {limit}"),
            ))));
        }
        Ok(())
    }
}
