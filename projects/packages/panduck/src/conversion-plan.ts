/**
 * ConversionPlan — Panduck 的产品级转换编排器。
 *
 * 与旧 DOM 规则链的区别：
 *   - DOM 级 CSS selector / mutation 不是 Panduck 的职责，相关入口已删除。
 *   - ConversionPlan 面向 **notedown-ir DocumentGraph**，是正式的转换合同。
 *     transforms 作用在 IR 语义节点上，output/publish 是产品级交付策略。
 *
 * 当前状态：
 *   - defineConversion 签名 + 类型已就位，可声明完整 plan。
 *   - transforms 由 Rust 侧 `panduck-convert::transform` 在 `DocumentGraph`
 *     上真实执行。已实现 pass：`replace-text`、`remove-blocks`。
 *     未实现的名字会被跳过并产生 `panduck.transform.unsupported-pass` 诊断。
 *   - output/publish 通过 Converter 分流到现有 Rust conversion 管线。
 *
 * 四层合同（与设计文档一致）：
 *   ExtractionProfile  → Pandark — "网页中取什么"
 *   DocumentTransform  → Notedown IR — "语义文档怎么改"（RewriteResult 有限返回值）
 *   ConversionPlan     → Panduck — "如何完成一次格式转换"（本文件）
 */

import type { DiagnosticRecord } from "./converter.js";

// ---------------------------------------------------------------------------
// Input — 输入格式路由
// ---------------------------------------------------------------------------

/** 已知输入格式标识符。v0 only —— 与 notedown-formats capability 对齐。 */
export type KnownInputFormat =
    | "html"
    | "markdown"
    | "docx"
    | "epub"
    | "pdf";

export type InputRoute = {
    /** 输入格式。Panduck 通过这个路由到 notedown-formats 的 import 模块。 */
    readonly format: KnownInputFormat;
    /** 可选 label —— 写入 provenance / report。 */
    readonly label?: string;
    /** 可选 URL hint —— 用于相对资产路径解析。 */
    readonly sourceUrl?: string;
};

// ---------------------------------------------------------------------------
// Transforms — IR 语义变换
// ---------------------------------------------------------------------------

/**
 * DocumentTransform 描述一个 IR 变换 pass。
 *
 * v0：`transforms` 序列化到 plan descriptor（`handlersJson` 参数），由 Rust
 * 侧按 `name` 匹配内置 pass 并在 `DocumentGraph` 上执行。已实现：
 *
 *   - `replace-text`：`params.search` 与 `params.replacement`（默认空串）
 *     替换所有 `Inline::Text` 内容。
 *   - `remove-blocks`：`targetKinds` 列出要删除的 block kind
 *     （`section` / `paragraph` / `list` / `table` / `quote` / `code` /
 *     `math` / `opaque`），匹配节点的子树一并清理。
 *
 * 未实现的名字会被跳过并产生 `panduck.transform.unsupported-pass` 诊断。
 * 每个 pass 的执行结果写入 report 的 `pipeline.transforms`。
 */
export type TransformPass = {
    /** 稳定名称 — 写入 passDescriptor + 诊断，并匹配 Rust 内置 pass。 */
    readonly name: string;
    /** 可选 traversal 方向 — 默认 bottom-up。 */
    readonly traversal?: "top-down" | "bottom-up";
    /** 可选 predicate —— 只对匹配的 block kind 执行变换。 */
    readonly targetKinds?: readonly string[];
    /**
     * Pass 参数。序列化后由 Rust 内置 pass 读取；schema 随 pass 名称定义，
     * 例如 `replace-text` 的 `{ search, replacement }`。
     */
    readonly params?: Readonly<Record<string, unknown>>;
    /**
     * 保留字段 —— 不参与序列化，也不会执行。执行由 Rust 侧按 `name` 匹配。
     * 声明该字段会得到 `panduck.transform.non-executable-handler` 警告诊断，
     * 而不会静默改变行为。
     */
    readonly pass?: unknown;
    /** 描述这个 transform 做什么 — 写入 passDescriptor。 */
    readonly description?: string;
};

// ---------------------------------------------------------------------------
// Output — 目标格式
// ---------------------------------------------------------------------------

/** 已知输出格式标识符。v0 已接线 markdown / epub。 */
export type KnownOutputFormat = "markdown" | "epub";

export type OutputRoute = {
    /** 目标格式。Panduck 通过这个路由到 notedown-formats 的 export 模块。 */
    readonly format: KnownOutputFormat;
    /** 可选 loss policy。默认 "warn"；"deny" 时任何 loss 都让 status = failed。 */
    readonly lossPolicy?: "warn" | "deny";
};

// ---------------------------------------------------------------------------
// Publish — 项目发布策略
// ---------------------------------------------------------------------------

/**
 * 发布策略。
 *
 * v0：只声明 `markdown-project`（单文件时等价于不发布，直接输出 markdown）。
 * 多文件、多文档策略留待后续合同。
 *
 * 目录发布在 Rust 侧始终是原子的（先写同盘 staging 目录，再 rename 到目标），
 * 没有非原子模式。远程资产下载不属于 Panduck：资产只能来自输入容器本身，
 * 由 Notedown 的 markdown-project writer materialize。因此这里不声明
 * `atomic` / `downloadAssets` 之类的旗标，避免出现接受但不生效的参数。
 */
export type PublishStrategy =
    | { readonly kind: "none" }
    | {
          readonly kind: "markdown-project";
          /** 输出目录（绝对或相对输入）。 */
          readonly directory: string;
      };

// ---------------------------------------------------------------------------
// ConversionPlan — 最终可执行的转换计划
// ---------------------------------------------------------------------------

/** 完整的 ConversionPlan —— defineConversion() 的返回值。 */
export type ConversionPlan = {
    readonly version: "panduck-conversion-v0";
    readonly id: string;
    readonly input: InputRoute;
    readonly transforms: readonly TransformPass[];
    readonly output: OutputRoute;
    readonly publish: PublishStrategy;
};

// ---------------------------------------------------------------------------
// defineConversion — 公开声明式入口
// ---------------------------------------------------------------------------

/** defineConversion 的配置对象。 */
export type ConversionConfig = {
    readonly input: InputRoute;
    readonly transforms?: readonly TransformPass[];
    readonly output: OutputRoute;
    readonly publish?: PublishStrategy;
    /** 可选稳定 ID —— 不传则自动生成。 */
    readonly id?: string;
};

let anonConversionCounter = 0;

/**
 * 声明式定义一次格式转换计划。
 *
 * 示例：
 *   const plan = defineConversion({
 *     input:  { format: "docx" },
 *     transforms: [
 *       { name: "normalize-headings", description: "flatten heading nesting" },
 *       { name: "resolve-references" }
 *     ],
 *     output: { format: "markdown" },
 *     publish: { kind: "markdown-project", directory: "./out" }
 *   });
 */
export function defineConversion(config: ConversionConfig): ConversionPlan {
    if (config.output.format !== "markdown" && config.output.format !== "epub") {
        throw new TypeError(
            `ConversionPlan output.format must be "markdown" or "epub" (got ${JSON.stringify(config.output.format)}) — v0 has no other writer`,
        );
    }
    const id = config.id ?? `conv-${++anonConversionCounter}`;
    return {
        version: "panduck-conversion-v0",
        id,
        input: config.input,
        transforms: config.transforms ?? [],
        output: config.output,
        publish: config.publish ?? { kind: "none" },
    };
}

// ---------------------------------------------------------------------------
// 辅助：把 ConversionPlan 序列化为 Rust 侧 plan descriptor
// ---------------------------------------------------------------------------

/**
 * 把 ConversionPlan 的 transforms + output 序列化为 JSON 字符串，
 * 传给 Rust 侧 wire contract。Rust 端只读取 `transforms` 并按 name 匹配
 * 已实现的 pass；未实现的 pass 会记录 diagnostic 并被跳过。
 */
export function serializeConversionPlan(
    plan: ConversionPlan,
): string {
    return JSON.stringify({
        version: plan.version,
        id: plan.id,
        input: { format: plan.input.format, label: plan.input.label, sourceUrl: plan.input.sourceUrl },
        transforms: plan.transforms.map((t) => ({
            name: t.name,
            traversal: t.traversal,
            targetKinds: t.targetKinds,
            params: t.params,
            description: t.description,
        })),
        output: {
            format: plan.output.format,
            lossPolicy: plan.output.lossPolicy ?? "warn",
        },
        publish: plan.publish,
    });
}

/** ConversionPlan 执行结果（由 Converter 返回，结构复用 ConverterResult）。 */
export type ConversionResult = {
    status: "success" | "success_with_loss" | "failed";
    text: string;
    diagnostics: DiagnosticRecord[];
    reportJson: string;
    /** 执行后的 publish 产物路径（如果有）。 */
    publishPath?: string;
};
