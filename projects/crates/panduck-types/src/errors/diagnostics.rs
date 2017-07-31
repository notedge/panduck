#![doc = include_str!("../readme.md")]

use crate::{PanduckError, PanduckErrorKind};
use std::{
    fmt::Debug,
    ops::{ControlFlow, FromResidual, Try},
    panic::Location,
};
use tracing::Level;

/// Gaia 诊断结果容器
///
/// 该结构体用于包装操作的结果，同时收集相关的诊断信息（错误、警告、跟踪信息）。
/// 它实现了 Rust 的 `Try` trait，可以与 `?` 操作符一起使用。
///
/// # 类型参数
/// - `T`: 操作的成功结果类型
///
/// # 示例
/// ```rust
/// use gaia_types::errors::diagnostics::GaiaDiagnostics;
///
/// fn parse_source() -> GaiaDiagnostics<String> {
///     let mut diagnostics = GaiaDiagnostics::success("parsed content".to_string());
///
///     // 添加警告
///     diagnostics.add_warning("Deprecated syntax detected");
///
///     diagnostics
/// }
/// ```
#[derive(Debug)]
pub struct PanduckDiagnostics<T> {
    /// 包含实际结果的结果类型，可能是成功值或 Fatal 错误值
    ///
    /// - 如果为 `Ok(T)`，表示操作成功完成
    /// - 如果为 `Err(GaiaError)`，表示发生了致命错误
    pub result: Result<T, PanduckError>,

    /// 包含诊断信息的错误向量
    ///
    /// 当操作失败时，可能会包含多个诊断错误，每个错误都包含了具体的错误信息和位置。
    /// 诊断信息可以包括错误、警告和跟踪信息，通过不同的日志级别进行区分。
    pub diagnostics: Vec<PanduckError>,
}

impl<T> PanduckDiagnostics<T> {
    /// 创建一个成功的诊断结果
    ///
    /// # 参数
    /// - `value`: 操作的成功结果值
    ///
    /// # 返回值
    /// 返回一个包含成功结果且没有诊断信息的 `GaiaDiagnostics` 实例
    ///
    /// # 示例
    /// ```rust
    /// let diagnostics = GaiaDiagnostics::success(42);
    /// assert!(diagnostics.result.is_ok());
    /// ```
    pub fn success(value: T) -> Self {
        Self { result: Ok(value), diagnostics: Vec::new() }
    }

    /// 创建一个失败的诊断结果
    ///
    /// # 参数
    /// - `fatal`: 导致操作失败的致命错误
    ///
    /// # 返回值
    /// 返回一个包含致命错误且没有额外诊断信息的 `GaiaDiagnostics` 实例
    ///
    /// # 示例
    /// ```rust
    /// use gaia_types::errors::GaiaError;
    ///
    /// let error = GaiaError::syntax_error("Invalid syntax", (1, 1));
    /// let diagnostics = GaiaDiagnostics::failure(error);
    /// assert!(diagnostics.result.is_err());
    /// ```
    pub fn failure(fatal: PanduckError) -> Self {
        Self { result: Err(fatal), diagnostics: Vec::new() }
    }

    /// 添加警告信息到诊断结果中
    ///
    /// # 参数
    /// - `warnings`: 可以转换为 `GaiaError` 的警告信息
    ///
    /// # 说明
    /// 警告信息表示非致命的问题，不会中断编译过程，但需要用户注意
    ///
    /// # 示例
    /// ```rust
    /// let mut diagnostics = GaiaDiagnostics::success(());
    /// diagnostics.add_warning("Unused variable 'x'");
    /// ```
    pub fn add_warning(&mut self, warnings: impl Into<PanduckError>) {
        let mut error = warnings.into();
        error.level = Level::WARN;
        self.diagnostics.push(error);
    }

    /// 添加错误信息到诊断结果中
    ///
    /// # 参数
    /// - `error`: 可以转换为 `PanduckError` 的错误信息
    ///
    /// # 说明
    /// 错误信息表示编译过程中的致命问题，会导致编译失败
    ///
    /// # 示例
    /// ```rust
    /// use gaia_types::errors::diagnostics::GaiaDiagnostics;
    /// use gaia_types::PanduckError;
    ///
    /// let mut diagnostics = GaiaDiagnostics::success(());
    /// diagnostics.add_error(PanduckError::syntax_error("Unexpected token", (1, 5)));
    /// ```
    pub fn add_error(&mut self, error: impl Into<PanduckError>) {
        let mut error = error.into();
        error.level = Level::ERROR;
        self.diagnostics.push(error);
    }

    /// 添加跟踪信息到诊断结果中
    ///
    /// # 参数
    /// - `tracing`: 可以转换为 `GaiaError` 的跟踪信息
    ///
    /// # 说明
    /// 跟踪信息用于调试目的，通常包含详细的内部状态信息
    ///
    /// # 示例
    /// ```rust
    /// let mut diagnostics = GaiaDiagnostics::success(());
    /// diagnostics.add_tracing("Entering function parse_expression");
    /// ```
    pub fn add_tracing(&mut self, tracing: impl Into<PanduckError>) {
        let mut error = tracing.into();
        error.level = Level::TRACE;
        self.diagnostics.push(error);
    }

    /// 检查是否应该中断执行
    ///
    /// # 返回值
    /// - `true`: 应该中断执行（存在致命错误或错误级别的诊断信息）
    /// - `false`: 可以继续执行
    ///
    /// # 中断规则
    /// 1. 如果 `result` 字段是 `Err`（致命错误）-> 中断
    /// 2. 如果诊断信息中包含 `ERROR` 级别的错误 -> 中断
    /// 3. 其他情况（只有警告或跟踪信息）-> 可以继续
    ///
    /// # 示例
    /// ```rust
    /// let diagnostics = GaiaDiagnostics::success(());
    /// assert!(!diagnostics.should_halt());
    /// ```
    pub fn should_halt(&self) -> bool {
        if self.result.is_err() {
            return true;
        }

        self.diagnostics.iter().any(|diag| diag.level == Level::ERROR)
    }

    /// 获取结果值，如果应该中断则返回 ControlFlow::Break
    ///
    /// # 返回值
    /// - `ControlFlow::Continue(T)`: 可以继续执行，返回成功值
    /// - `ControlFlow::Break(Self)`: 应该中断，返回完整的诊断信息
    ///
    /// # 说明
    /// 这个方法用于实现 `Try` trait，支持 `?` 操作符的使用
    ///
    /// # 示例
    /// ```rust
    /// let diagnostics = GaiaDiagnostics::success(42);
    /// match diagnostics.try_value() {
    ///     ControlFlow::Continue(value) => println!("Success: {}", value),
    ///     ControlFlow::Break(diag) => eprintln!("Failed: {:?}", diag),
    /// }
    /// ```
    pub fn try_value(self) -> ControlFlow<Self, T> {
        if self.should_halt() {
            ControlFlow::Break(self)
        }
        else {
            match self.result {
                Ok(value) => ControlFlow::Continue(value),
                Err(_) => ControlFlow::Break(self), // 这行理论上不会执行，因为 should_halt 已经检查了
            }
        }
    }
}

/// 为 `GaiaDiagnostics` 实现 `FromResidual` trait
///
/// 这个实现允许在 `?` 操作符中使用 `GaiaDiagnostics`，支持错误传播
impl<T, U> FromResidual<PanduckDiagnostics<T>> for PanduckDiagnostics<U> {
    /// 从残差（residual）值创建新的诊断结果
    ///
    /// # 参数
    /// - `residual`: 前一个操作的诊断结果残差
    ///
    /// # 返回值
    /// 返回一个新的 `GaiaDiagnostics<U>` 实例
    ///
    /// # 行为
    /// 1. 如果残差中没有致命错误但包含错误级别的诊断信息，创建阶段错误
    /// 2. 如果残差中已有致命错误，直接传递错误和诊断信息
    /// 3. 成功的情况不应该进入此方法（逻辑错误）
    #[track_caller]
    fn from_residual(residual: PanduckDiagnostics<T>) -> Self {
        match residual.result {
            // 没有致命错误，检查诊断中是否有错误级别的问题
            Ok(_) => {
                let has_error = residual.diagnostics.iter().any(|diag| diag.level == Level::ERROR);
                // 有错误，创建阶段错误
                if has_error {
                    let location = Location::caller();
                    Self {
                        result: Err(PanduckErrorKind::StageError { location: *location }.into()),
                        diagnostics: residual.diagnostics,
                    }
                }
                // 这应该是一个逻辑错误，因为成功的情况不应该进入 from_residual
                else {
                    unreachable!()
                }
            }
            // 已经有致命错误，直接传递错误和诊断信息
            Err(e) => Self { result: Err(e), diagnostics: residual.diagnostics },
        }
    }
}

/// 为 `GaiaDiagnostics` 实现 `Try` trait
///
/// 这个实现使得 `GaiaDiagnostics` 可以与 `?` 操作符一起使用，
/// 支持在编译过程中进行错误传播和诊断信息收集
impl<T> Try for PanduckDiagnostics<T> {
    /// 成功时的输出类型
    type Output = T;

    /// 失败时的残差类型
    type Residual = PanduckDiagnostics<T>;

    /// 从输出值创建成功的诊断结果
    ///
    /// # 参数
    /// - `output`: 成功的输出值
    ///
    /// # 返回值
    /// 返回一个包含成功结果的 `GaiaDiagnostics` 实例
    fn from_output(output: Self::Output) -> Self {
        Self::success(output)
    }

    /// 分支操作，决定是继续执行还是中断
    ///
    /// # 返回值
    /// - `ControlFlow::Continue(T)`: 继续执行，返回成功值
    /// - `ControlFlow::Break(Self::Residual)`: 中断执行，返回诊断信息
    ///
    /// # 说明
    /// 这个方法被 `?` 操作符调用，用于决定是否传播错误
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        self.try_value()
    }
}
