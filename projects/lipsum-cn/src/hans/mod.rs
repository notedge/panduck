use std::{
    collections::{BTreeSet, HashMap},
    mem::{take, transmute},
};

use dynfmt::{Format, SimpleCurlyFormat};
use rand::{rngs::SmallRng, seq::IteratorRandom, Rng, SeedableRng};

//noinspection NonAsciiCharacters
#[derive(Debug, Clone)]
pub struct 检讨生成器 {
    rng: SmallRng,
    名言: BTreeSet<&'static str>,
    说过: BTreeSet<&'static str>,
    启发: BTreeSet<&'static str>,
    垫话: BTreeSet<&'static str>,
    垫话可终结: BTreeSet<&'static str>,
}

impl Default for 检讨生成器 {
    fn default() -> Self {
        let mut raw = Self {
            rng: SmallRng::from_entropy(),
            名言: Default::default(),
            说过: Default::default(),
            启发: Default::default(),
            垫话: Default::default(),
            垫话可终结: Default::default(),
        };
        raw.loading();
        raw
    }
}

//noinspection NonAsciiCharacters
impl 检讨生成器 {
    pub fn 设置随机数(&mut self, 种子: [u128; 2]) {
        let seed = unsafe { transmute::<[u128; 2], [u8; 32]>(种子) };
        self.rng = SmallRng::from_seed(seed)
    }
    pub fn 生成文章(&mut self, 主题: &str, 字数要求: usize) -> String {
        let mut 总字数 = String::with_capacity(字数要求 + 100);
        let mut 段落字数 = String::with_capacity(500);
        let mut 段落可终结 = false;
        let mut 段落起始 = true;
        while 总字数.len() + 段落字数.len() < 字数要求 {
            // 段落
            match self.rng.gen_range(0..100) {
                // 强制终结
                _ if 段落字数.len() >= 400 && 段落可终结 => {
                    总字数.push_str(&take(&mut 段落字数));
                    总字数.push_str("\n\n");
                    段落可终结 = false
                }
                // 生成章节
                n if n <= 5 && 段落字数.len() >= 200 && 段落可终结 => {
                    总字数.push_str(&take(&mut 段落字数));
                    总字数.push_str("\n\n");
                    段落可终结 = false
                }
                // 生成名言
                n if n <= 25 => {
                    let 说过 = self.生成说过();
                    let 启发 = self.生成启发();
                    段落字数.push_str(&self.生成名言(&说过, &启发));
                    段落可终结 = true
                }
                // 生成废话并终结段落
                n if n <= 50 => {
                    段落字数.push_str(&self.生成垫话(主题, true));
                    段落可终结 = true
                }
                // 生成废话
                _ => {
                    段落字数.push_str(&self.生成垫话(主题, false));
                    段落可终结 = false
                }
            }
        }
        总字数.push_str(&take(&mut 段落字数));
        总字数.push_str("\n\n");
        return 总字数;
    }
}

//noinspection NonAsciiCharacters
impl 检讨生成器 {
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
    pub fn 生成名言(&mut self, 说过: &str, 启发: &str) -> String {
        let t = self.名言.iter().choose(&mut self.rng).unwrap();
        let mut map = HashMap::new();
        map.insert("说过", 说过);
        map.insert("启发", 启发);
        let out = SimpleCurlyFormat.format(t, map);
        out.unwrap().to_string()
    }
    pub fn 生成垫话(&mut self, 主题: &str, 终结段落: bool) -> String {
        let t = match 终结段落 {
            true => self.垫话可终结.iter().choose(&mut self.rng).unwrap(),
            false => self.垫话.iter().choose(&mut self.rng).unwrap(),
        };
        let mut map = HashMap::new();
        map.insert("主题", 主题);
        let out = SimpleCurlyFormat.format(t, map);
        out.unwrap().to_string()
    }
    pub fn 生成启发(&mut self) -> String {
        let out = self.垫话.iter().choose(&mut self.rng).unwrap();
        out.to_string()
    }
    pub fn 生成说过(&mut self) -> String {
        let out = self.说过.iter().choose(&mut self.rng).unwrap();
        out.to_string()
    }
}

#[test]
fn test() {
    let mut r = 检讨生成器::default();
    println!("{}", r.生成文章("啊", 6000))
}
