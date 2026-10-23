/**
 * PassDescriptor — 一个转换 pass 的能力与边界声明。
 *
 * 用于在 ExtractionProfile / DocumentTransform / RendererPolicy 等
 * 跨模块配置之间声明自己的 I/O 形状、遍历方向、可观测副作用。
 *
 * 设计约束：
 *   - Capability 是声明式的，不是运行时强制的。宿主（Converter）会
 *     根据 capability 决定把哪个数据面传给这个 pass。
 *   - deterministic = false 的 pass 必须显式声明；宿主可以选择
 *     在诊断里标红，或者在缓存策略里跳过它。
 */

export type Capability = "read-page" | "modify-page" | "modify-ir" | "write-report";

export type PassDescriptor = {
    /** Pass 名 — 用于诊断、报告、缓存键。 */
    readonly name: string;
    /** 输入数据面：HTML page 或 Notedown document。 */
    readonly input: "page" | "document";
    /** 输出数据面：page 或 document（中间 pass），或 report（最终 pass）。 */
    readonly output: "page" | "document" | "report";
    /** IR 遍历方向；page 输入的 pass 通常不适用，填 top-down 作为约定。 */
    readonly traversal: "top-down" | "bottom-up" | "typewise";
    /** 同一输入是否必然得到相同输出。false 的 pass 宿主会特别对待。 */
    readonly deterministic: boolean;
    /** 该 pass 声明自己会用到的 capability 集合。必须至少有一个。 */
    readonly capabilities: readonly Capability[];
};
