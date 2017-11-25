//! Unified entry for the Panduck document conversion family.
//!
//! Enable capabilities through crate features. Each module re-exports the
//! corresponding `panduck-*` crate without copying types or adding global
//! state.

#![deny(missing_docs)]

#[cfg(feature = "types")]
pub mod types {
    //! Shared conversion contracts, adapter errors, and I/O helpers.
    pub use panduck_types::*;
}

#[cfg(feature = "diagnostic")]
pub mod diagnostic {
    //! Panduck diagnostics on the shared `boost-diagnostic` contract.
    pub use panduck_diagnostic::*;
}

#[cfg(feature = "convert")]
pub mod convert {
    //! Conversion orchestration over `notedown-ir`.
    pub use panduck_convert::*;
}
