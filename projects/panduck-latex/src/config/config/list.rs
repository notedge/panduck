use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListConfig {
    pub ordered_style: OrderedStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderedStyle {
    Unset,
    Alpha,
    AlphaLower,
    Roman,
    RomanLower,
    Arabic,
}

impl Default for ListConfig {
    fn default() -> Self {
        Self { ordered_style: Default::default() }
    }
}

impl Default for OrderedStyle {
    fn default() -> Self {
        Self::Unset
    }
}

impl OrderedStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unset => "",
            Self::Alpha => "[A]",
            Self::AlphaLower => "a",
            Self::Roman => "[I]",
            Self::RomanLower => "[i]",
            Self::Arabic => "[1]",
        }
    }
}
