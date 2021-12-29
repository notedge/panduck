use std::{
    collections::{BTreeSet, HashMap},
    mem::transmute,
};

use dynfmt::{Format, SimpleCurlyFormat};
use rand::{rngs::SmallRng, seq::IteratorRandom, Rng, SeedableRng};

#[derive(Debug, Clone)]
pub struct Builder {
    rng: SmallRng,
    saying: BTreeSet<&'static str>,
    saied: BTreeSet<&'static str>,
    idea: BTreeSet<&'static str>,
    bosh: BTreeSet<&'static str>,
}

impl Default for Builder {
    fn default() -> Self {
        let mut raw = Self {
            rng: SmallRng::from_entropy(),
            saying: Default::default(),
            saied: Default::default(),
            idea: Default::default(),
            bosh: Default::default(),
        };
        raw.loading();
        raw
    }
}

impl Builder {
    pub fn 设置随机数(&mut self, 种子: [u128; 2]) {
        let seed = unsafe { transmute::<[u128; 2], [u8; 32]>(种子) };
        self.rng = SmallRng::from_seed(seed)
    }
    pub fn 生成(&mut self, 主题: &str, 字数要求: usize) -> String {
        let mut out = String::with_capacity(字数要求 + 100);

        while out.len() < 字数要求 {
            match self.rng.gen_range(0..100) {
                n if n <= 5 => out.push_str("\n\n"),
                n if n <= 25 => {
                    let a = self.生成说过();
                    let b = self.生成启发();
                    out.push_str(&self.生成名言(&a, &b))
                }
                _ => out.push_str(&self.生成垫话(主题)),
            }
        }
        return out;
    }
}

impl Builder {
    fn loading(&mut self) {
        for s in include_str!("说过.txt").lines() {
            let text = s.trim();
            if !text.is_empty() {
                self.saied.insert(text);
            }
        }
        for s in include_str!("名言.txt").lines() {
            let text = s.trim();
            if !text.is_empty() {
                self.saying.insert(text);
            }
        }
        for s in include_str!("垫话.txt").lines() {
            let text = s.trim();
            if !text.is_empty() {
                self.bosh.insert(text);
            }
        }
        for s in include_str!("启发.txt").lines() {
            let text = s.trim();
            if !text.is_empty() {
                self.idea.insert(text);
            }
        }
    }
    pub fn 生成名言(&mut self, 说过: &str, 启发: &str) -> String {
        let t = self.saying.iter().choose(&mut self.rng).unwrap();
        let mut map = HashMap::new();
        map.insert("说过", 说过);
        map.insert("启发", 启发);
        let out = SimpleCurlyFormat.format(t, map);
        out.unwrap().to_string()
    }
    pub fn 生成垫话(&mut self, 主题: &str) -> String {
        let t = self.bosh.iter().choose(&mut self.rng).unwrap();
        let mut map = HashMap::new();
        map.insert("主题", 主题);
        let out = SimpleCurlyFormat.format(t, map);
        out.unwrap().to_string()
    }
    pub fn 生成启发(&mut self) -> String {
        let out = self.bosh.iter().choose(&mut self.rng).unwrap();
        out.to_string()
    }
    pub fn 生成说过(&mut self) -> String {
        let out = self.saied.iter().choose(&mut self.rng).unwrap();
        out.to_string()
    }
}

#[test]
fn test() {
    let mut r = Builder::default();
    println!("{}", r.生成("啊", 100))
}
