#![doc = include_str!("readme.md")]

use crate::helpers::{SourcePosition, SourceText};
use crate::{reader::Token, PanduckDiagnostics, PanduckError};

pub trait TokenType: Copy {
    const END_OF_STREAM: Self;

    fn is_whitespace(&self) -> bool;

    fn is_ignored(&self) -> bool;
}

/// 词法分析器状态管理实用类
///
/// 这是一个通用的词法分析器状态管理器，提供了完整的词法分析功能，
/// 包括字符位置跟踪、token 收集、错误处理等。
///
/// # 设计目标
///
/// * **通用性**: 支持任意 token 类型，只要实现 `Copy` trait
/// * **性能**: 高效的字符迭代和位置跟踪
/// * **易用性**: 提供丰富的辅助方法简化词法分析
/// * **错误处理**: 集成 Gaia 错误系统
///
/// # 示例
///
/// ```rust
/// use gaia_types::{
///     lexer::LexerState,
///     reader::{SourcePosition, Token},
/// };
///
/// #[derive(Clone, Copy, Debug)]
/// enum MyToken {
///     Identifier,
///     Number,
///     Whitespace,
/// }
///
/// let input = "hello 123";
/// let mut state = LexerState::new(input);
///
/// // 添加 token
/// state.add_token(MyToken::Identifier, 0, 5, 1, 1);
/// state.add_token(MyToken::Whitespace, 5, 1, 1, 6);
/// state.add_token(MyToken::Number, 6, 3, 1, 7);
///
/// // 生成 token 流
/// let token_stream = state.into_token_stream();
/// ```
#[derive(Debug)]
pub struct LexerState<'input, T: TokenType> {
    source: &'input SourceText,
    /// 收集的 tokens
    tokens: Vec<Token<T>>,
    /// 当前字节偏移量（从 0 开始）
    offset: usize,
    diagnostics: Vec<PanduckError>,
}

impl<'input, T: TokenType> LexerState<'input, T> {
    pub fn new(input: &'input SourceText) -> Self {
        Self {
            source: input,
            tokens: Vec::new(),
            offset: 0,
            diagnostics: vec![],
        }
    }

    pub fn current_char(&self) -> Option<char> {
        self.source.get_char(self.offset).ok()
    }

    pub fn peek_char(&self) -> Option<char> {
        self.source
            .get_char(self.offset + self.current_char().map_or(0, |c| c.len_utf8()))
            .ok()
    }

    pub fn advance(&mut self) {
        if let Some(c) = self.current_char() {
            self.offset += c.len_utf8();
        }
    }

    pub fn add_token(&mut self, token_type: T, start_offset: usize) {
        let position = SourcePosition {
            offset: start_offset,
            length: self.offset - start_offset,
        };
        self.tokens.push(Token {
            token_type,
            position,
        });
    }

    pub fn success(mut self) -> PanduckDiagnostics<Vec<Token<T>>> {
        let position = SourcePosition {
            offset: self.source.utf8_length(),
            length: 0,
        };
        self.tokens.push(Token {
            token_type: T::END_OF_STREAM,
            position,
        });
        PanduckDiagnostics {
            result: Ok(self.tokens),
            diagnostics: self.diagnostics,
        }
    }

    
    
    pub fn push_errors(&mut self, errors: &mut Vec<PanduckError>) {
        errors.append(&mut self.diagnostics);
    }

    pub fn take_errors(&mut self) -> Vec<PanduckError> {
        std::mem::take(&mut self.diagnostics)
    }


    pub fn failure(self, fatal: PanduckError) -> PanduckDiagnostics<Vec<Token<T>>> {
        PanduckDiagnostics {
            result: Err(fatal),
            diagnostics: self.diagnostics,
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}
