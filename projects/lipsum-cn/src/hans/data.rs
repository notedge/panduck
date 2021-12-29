use super::*;

//noinspection NonAsciiCharacters
#[derive(Debug, Clone)]
pub struct Data {
    pub 名言: BTreeSet<&'static str>,
    pub 说过: BTreeSet<&'static str>,
    pub 启发: BTreeSet<&'static str>,
    pub 垫话: BTreeSet<&'static str>,
    pub 垫话可终结: BTreeSet<&'static str>,
}

impl Default for Data {
    fn default() -> Self {
        let mut out = Self {
            名言: Default::default(),
            说过: Default::default(),
            启发: Default::default(),
            垫话: Default::default(),
            垫话可终结: Default::default(),
        };
        out.loading();
        out
    }
}

impl Data {
    fn loading(&mut self) {
        for s in include_str!("说过.txt").lines() {
            let text = s.trim();
            if !text.is_empty() {
                self.说过.insert(text);
            }
        }
        for s in include_str!("名言.txt").lines() {
            let text = s.trim();
            if !text.is_empty() {
                self.名言.insert(text);
            }
        }
        for s in include_str!("垫话.txt").lines() {
            let text = s.trim();
            if !text.is_empty() {
                self.垫话.insert(text);
            }
        }
        for s in include_str!("垫话可终结.txt").lines() {
            let text = s.trim();
            if !text.is_empty() {
                self.垫话可终结.insert(text);
            }
        }
        for s in include_str!("启发.txt").lines() {
            let text = s.trim();
            if !text.is_empty() {
                self.启发.insert(text);
            }
        }
    }
}
