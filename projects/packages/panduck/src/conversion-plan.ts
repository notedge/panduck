/**
 * ConversionPlan — Panduck 的产品级转换编排器。
 *
 * 与 RuleBuilder 的区别：
 *   - RuleBuilder 面向 HTML DOM（linkedom CSS selector + DOM mutation），
 *     是 Panduck 早期在 Notedown IR 还没就绪前的过渡方案。
 *   - ConversionPlan 面向 **notedown-ir DocumentGraph**，是正式的转换合同。
 *     transforms 作用在 IR 语义节点上，output/publish 是产品级交付策略。
 *
 * 当前状态（v0 骨架）：
 *   - defineConversion 签名 + 类型已就位，可声明完整 plan。
 *   - transforms 暂不执行（Rust 侧尚未暴露 DocumentGraph 到 TS）。
 *   - output/publish 通过 Converter 分流到现有 Rust conversion 管线。
 *   - RuleBuilder 继续可用，标记 @deprecated —— 等 IR transform pass 落地后删除。
 *
 * 四层合同（与设计文档一致）：
 *   ExtractionProfile  → Pandark — "网页中取什么"
 *   DocumentTransform  → Notedown IR — "语义文档怎么改"（RewriteResult 有限返回值）
 *   ConversionPlan     → Panduck — "如何完成一次格式转换"（本文件）
 *   RendererPolicy     → Panduck — "目标格式怎么写"
 */

import type { MarkdownPolicy } from "./renderer-policy.js";
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
 * v0：transforms 只作声明，不真正执行。Rust 侧有 notedown-ir walk_mut +
 * RewriteResult 基础设施（见 `notedown/src/transform_errors.rs` + `walk.rs`），
 * 但尚未暴露到 Panduck TS。当前把 transforms 序列化到 handlersJson 传给
 * Rust，让 wire contract 先就位。
 *
 * v1（待 Rust 落地后）：每个 TransformPass 会在 DocumentGraph 上跑一次
 * walk_mut，transform 闭包接受 NodeId + NodeView（TS 侧 IR 视图），返回
 * RewriteResult —— 与 notedown-ir 的 RewriteResult 有限返回值对齐
 * （Keep / Replace / Remove / Unwrap / SkipChildren / Error）。
 */
export type TransformPass = {
    /** 稳定名称 — 写入 passDescriptor + 诊断。 */
    readonly name: string;
    /** 可选 traversal 方向 — 默认 bottom-up。 */
    readonly traversal?: "top-down" | "bottom-up";
    /** 可选 predicate —— 只对匹配的 block kind 执行变换。 */
    readonly targetKinds?: readonly string[];
    /**
     * v0：闭包只作类型占位，不执行。v1 会接受 IR 节点视图 + 返回 RewriteResult。
     * 保留这个字段是为了让声明式 API 看起来完整，实际执行由 Rust 侧按 name 匹配。
     */
    readonly pass?: unknown;
    /** 描述这个 transform 做什么 — 写入 passDescriptor。 */
    readonly description?: string;
};

// ---------------------------------------------------------------------------
// Output — 目标格式 + RendererPolicy
// ---------------------------------------------------------------------------

/** 已知输出格式标识符。v0 only —— 与 notedown-formats export 对齐。 */
export type KnownOutputFormat = "markdown" | "html";

export type OutputRoute = {
    /** 目标格式。Panduck 通过这个路由到 notedown-formats 的 export 模块。 */
    readonly format: KnownOutputFormat;
    /** 可选 RendererPolicy —— 控制 writer 行为。 */
    readonly policy?: MarkdownPolicy;
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
 * v1：多文件、多文档、原子写出、资产 materialization 都在这里定义。
 */
export type PublishStrategy =
    | { readonly kind: "none" }
    | {
          readonly kind: "markdown-project";
          /** 输出目录（绝对或相对输入）。 */
          readonly directory: string;
          /** 是否 materialize 远程 assets 到本地。默认 false。 */
          readonly downloadAssets?: boolean;
          /** 原子写出策略。默认 true —— 写到临时目录再 rename。 */
          readonly atomic?: boolean;
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
 *     output: { format: "markdown", policy: markdownGfm() },
 *     publish: { kind: "markdown-project", directory: "./out" }
 *   });
 */
export function defineConversion(config: ConversionConfig): ConversionPlan {
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
// 辅助：把 ConversionPlan 序列化到 handlersJson
// ---------------------------------------------------------------------------

/**
 * 把 ConversionPlan 的 transforms + output 序列化为 JSON 字符串，
 * 传给 Rust 侧 wire contract。Rust 端按 name 匹配已实现的 transform pass；
 * 未实现的 pass 会记录 diagnostic 并被跳过。
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
            description: t.description,
        })),
        output: {
            format: plan.output.format,
            policyName: plan.output.policy?.name,
            lossPolicy: plan.output.lossPolicy ?? "warn",
        },
        publish: plan.publish,
    });
}

/** ConversionPlan 执行结果（由 Converter 返回，结构复用 ConverterResult）。 */
export type ConversionResult = {
    status: "success" | "success_with_loss" | "success_unvalidated" | "failed";
    text: string;
    diagnostics: DiagnosticRecord[];
    reportJson: string;
    /** 执行后的 publish 产物路径（如果有）。 */
    publishPath?: string;
};
