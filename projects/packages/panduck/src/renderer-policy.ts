/**
 * RendererPolicy — 目标格式写作策略层。
 *
 * 与 Pandark 的 ExtractionProfile（"网页中取什么"）和 Notedown IR 的
 * DocumentTransform（"语义文档怎么改"）并列，RendererPolicy 解决的是
 * "目标格式怎么写"。
 *
 * 这个文件只声明策略类型与辅助构造函数。策略本身 **不** 触发渲染；
 * 它被 Converter 在 writer 阶段读取，用来改写或替换 Notedown IR block
 * 的默认 Markdown 输出。
 *
 * 与 DOM 规则链的边界：
 *   - DOM 级选择/裁剪/替换不属于 Panduck；相关 TS 入口已删除，转换走 Rust lowering。
 *   - RendererPolicy.handlers 是 writer 级
 *   - RendererPolicy handler 不接受 DOM node，只接受 IR block kind + attributes
 *   - handler 返回值只能是 Markdown writer 能理解的结果：pass-through / fragment / suppress
 */

// ---------------------------------------------------------------------------
// Policy handler
// ---------------------------------------------------------------------------

/** 受控渲染回调 — 只接受 writer context，不接受原始 HTML 节点。 */
export type HandlerContext = {
    /** 当前 writer 正在处理的 IR block kind（Notedown IR variant 名）。 */
    blockKind: string;
    /** 可选：writer 能看到的属性（来自 IR block 的 attributes map）。 */
    attributes?: ReadonlyMap<string, string>;
};

/** 渲染策略 handler 的返回值。 */
export type PolicyHandlerResult =
    | { readonly kind: "pass-through" }           // writer 用默认行为
    | { readonly kind: "fragment"; readonly markdown: string }  // writer fragment，允许 Markdown 字符串
    | { readonly kind: "suppress" };               // writer 跳过这个 block

/** 策略 handler — 纯函数，不产生副作用。 */
export type PolicyHandler = (ctx: HandlerContext) => PolicyHandlerResult;

// ---------------------------------------------------------------------------
// MarkdownPolicy
// ---------------------------------------------------------------------------

/** Markdown writer 策略配置。 */
export type MarkdownPolicy = {
    /** 策略名称 — 用于诊断与报告。 */
    readonly name: string;
    /** 标题风格：ATX（`#`）或 Setext（`==` / `--`）。 */
    readonly heading: "atx" | "setext";
    /** 无序列表符号。 */
    readonly bullet: "-" | "*" | "+";
    /** 代码块是否使用围栏（```）；false 则缩进。 */
    readonly fencedCode: boolean;
    /** 遇到 writer 不支持的 IR block 时的行为。 */
    readonly unsupported: "report" | "drop" | "keep-opaque";
    /** 自定义 handler — key 匹配 IR block 的 kind 或 writer 的 handlerKey。 */
    readonly handlers?: Readonly<Record<string, PolicyHandler>>;
};

// ---------------------------------------------------------------------------
// ConverterConfig — Profile 化的转换器配置
// ---------------------------------------------------------------------------

/** Profile 化的转换器配置。 */
export type ConverterConfig = {
    /** 使用哪个 ExtractionProfile（来自 Pandark，跨系统边界）。 */
    readonly profileRef?: string;
    /** 页面 scope 选择（可选 override）。 */
    readonly scope?: {
        readonly candidates?: readonly string[];  // CSS selectors
        readonly exclude?: readonly string[];
    };
    /** 渲染策略。 */
    readonly rendererPolicy: MarkdownPolicy;
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** 构造一个默认 Markdown policy（GFM 风格）。 */
export function defineMarkdownPolicy(overrides: Partial<MarkdownPolicy> = {}): MarkdownPolicy {
    return {
        name: "gfm-default",
        heading: "atx",
        bullet: "-",
        fencedCode: true,
        unsupported: "report",
        handlers: {},
        ...overrides,
    };
}

/** 构造一个带自定义 handlers 的 policy。后续传入的 handlers 会覆盖同名键。 */
export function withHandlers(
    policy: MarkdownPolicy,
    handlers: Record<string, PolicyHandler>,
): MarkdownPolicy {
    return {
        ...policy,
        handlers: { ...(policy.handlers ?? {}), ...handlers },
    };
}
