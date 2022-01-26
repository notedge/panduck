use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListConfig {
    pub ordered_style: OrderedStyle,
    pub orderless_style: OrderlessStyle,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderlessStyle {
    Unset,
    Dot,
    Hyphen,
    BlackSquare,
}

impl Default for ListConfig {
    fn default() -> Self {
        Self { ordered_style: Default::default(), orderless_style: Default::default() }
    }
}

impl Default for OrderedStyle {
    fn default() -> Self {
        Self::Unset
    }
}

impl Default for OrderlessStyle {
    fn default() -> Self {
        Self::Unset
    }
}

impl OrderedStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unset => "",
            Self::Alpha => "[A]",
            Self::AlphaLower => "[a]",
            Self::Roman => "[I]",
            Self::RomanLower => "[i]",
            Self::Arabic => "[1]",
        }
    }
}

impl OrderlessStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unset => "",
            Self::Dot => "[]",
            Self::Hyphen => "[-]",
            Self::BlackSquare => "[$\\blacksquare$]",
        }
    }
}
