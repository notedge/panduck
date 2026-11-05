//! Math font pack owned by panduck.
//!
//! `document-fonts` ships no bytes, so the conversion host embeds the two math
//! faces it typesets with and installs them through the injection entry point.
//! The files live in `fonts/`, next to their GUST and OFL licence texts.

use document_fonts::{MathFontPack, install_math_fonts as install_pack};

const LATIN_MODERN_MATH: &[u8] = include_bytes!("../fonts/LatinModernMath-Regular.otf");
const XITS_MATH: &[u8] = include_bytes!("../fonts/XITSMath-Regular.otf");

/// Install the pack once per process. Repeated calls are a no-op.
pub(crate) fn install_math_fonts() {
    install_pack(&MathFontPack::new(LATIN_MODERN_MATH, XITS_MATH));
}
