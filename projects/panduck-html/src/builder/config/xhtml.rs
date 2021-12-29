use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum XHtml {
    True,
    False,
}

impl XHtml {
    pub fn tag(&self, s: &str) -> String {
        match &self {
            Self::True => format!("<{}>", s),
            Self::False => format!("<{}/>", s),
        }
    }
}
