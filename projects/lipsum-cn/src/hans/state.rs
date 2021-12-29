use super::*;

//noinspection NonAsciiCharacters
#[derive(Debug, Clone)]
pub struct State {
    pub rng: SmallRng,
    pub 文章字数: String,
    pub 段落字数: String,
    pub 段落可终结: bool,
    pub 段落起始: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            rng: SmallRng::from_entropy(),
            文章字数: String::with_capacity(10000),
            段落字数: String::with_capacity(500),
            段落可终结: false,
            段落起始: true,
        }
    }
}

impl State {
    pub fn reset(&mut self) {
        self.文章字数.clear();
        self.段落字数.clear();
        self.段落可终结 = false;
        self.段落起始 = true;
    }
    pub unsafe fn set_seed(&mut self, seed: (u128, u128)) {
        let seed = transmute::<(u128, u128), [u8; 32]>(seed);
        self.rng = SmallRng::from_seed(seed)
    }
    pub unsafe fn get_seed(&self) -> (u128, u128) {
        todo!()
    }
}
