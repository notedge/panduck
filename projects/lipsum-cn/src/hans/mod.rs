use std::{
    collections::{BTreeSet, HashMap},
    mem::{take, transmute},
};

use dynfmt::{Format, SimpleCurlyFormat};
use rand::{rngs::SmallRng, seq::IteratorRandom, Rng, SeedableRng};

use self::{data::Data, state::State};

mod data;
mod state;

//noinspection NonAsciiCharacters
#[derive(Debug, Default, Clone)]
pub struct 废话生成器 {
    data: Data,
    state: State,
}

//noinspection NonAsciiCharacters
impl 废话生成器 {
    pub fn 设置随机数(&mut self, 种子: (u128, u128)) {
        unsafe { self.state.set_seed(种子) }
    }
    pub fn 读取随机数(&mut self) -> (u128, u128) {
        unsafe { self.state.get_seed() }
    }
    pub fn 生成文章(&mut self, 主题: &str, 字数要求: usize) -> String {
        self.state.reset();
        while self.state.文章字数.len() + self.state.段落字数.len() < 字数要求 {
            match self.state.rng.gen_range(0..100) {
                // 强制终结段落
                _ if self.需要强制终结段落() => self.终结段落(),
                // 随机插入段落
                n if n <= 5 && self.可以终结段落() => self.终结段落(),
                // 段落开始, 插入名言
                _ if self.state.段落起始 => self.插入名言(),
                // 段落中间, 随机插入名言
                n if n <= 25 => self.插入名言(),
                // 生成废话并建议终结段落
                n if n <= 50 => self.插入垫话(主题, true),
                // 生成废话
                _ => self.插入垫话(主题, false),
            }
        }
        // 检查还在缓冲区段落
        self.终结段落();
        take(&mut self.state.文章字数)
    }
    #[inline]
    fn 插入名言(&mut self) {
        let a = self.生成说过();
        let b = self.生成启发();
        let c = &self.生成名言(&a, &b);
        self.state.段落字数.push_str(c);
        self.state.段落起始 = false;
        self.state.段落可终结 = true
    }
    #[inline]
    fn 插入垫话(&mut self, theme: &str, stop: bool) {
        let w = self.生成垫话(theme, stop);
        self.state.段落字数.push_str(&w);
        self.state.段落起始 = false;
        self.state.段落可终结 = stop
    }
    #[inline]
    fn 终结段落(&mut self) {
        self.state.文章字数.push_str(&take(&mut self.state.段落字数));
        self.state.文章字数.push_str("\n\n");
        self.state.段落起始 = true;
        self.state.段落可终结 = false
    }
    #[inline(always)]
    fn 可以终结段落(&self) -> bool {
        self.state.段落字数.len() >= 200 && self.state.段落可终结
    }
    #[inline(always)]
    fn 需要强制终结段落(&self) -> bool {
        self.state.段落字数.len() >= 400 && self.state.段落可终结
    }
}

//noinspection NonAsciiCharacters
impl 废话生成器 {
    pub fn 生成名言(&mut self, 说过: &str, 启发: &str) -> String {
        let t = self.data.名言.iter().choose(&mut self.state.rng).unwrap();
        let mut map = HashMap::new();
        map.insert("说过", 说过);
        map.insert("启发", 启发);
        let out = SimpleCurlyFormat.format(t, map);
        out.unwrap().to_string()
    }
    pub fn 生成垫话(&mut self, 主题: &str, 终结段落: bool) -> String {
        let t = match 终结段落 {
            true => self.data.垫话可终结.iter().choose(&mut self.state.rng),
            false => self.data.垫话.iter().choose(&mut self.state.rng),
        };
        let mut map = HashMap::new();
        map.insert("主题", 主题);
        let out = SimpleCurlyFormat.format(t.unwrap(), map);
        out.unwrap().to_string()
    }
    pub fn 生成启发(&mut self) -> String {
        let out = self.data.启发.iter().choose(&mut self.state.rng).unwrap();
        out.to_string()
    }
    pub fn 生成说过(&mut self) -> String {
        let out = self.data.说过.iter().choose(&mut self.state.rng).unwrap();
        out.to_string()
    }
}
