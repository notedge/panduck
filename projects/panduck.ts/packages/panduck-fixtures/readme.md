# @notedge/panduck-fixtures

Panduck 文档家族的语料家底。语料物理上只存在这里一份，`panduck-harness`、示例与未来下游消费者都只读引用。

## 落点约定

上游 crate 的测试代码不下移。下游只接三类东西：输入样例、期望输出、分类。

```text
fixtures/
  formats/<format>/          # document.formats：docx / epub / xlsx / pptx / markdown / tex / typst 样例（含整包二进制）
  convert/                   # document.convert：转换、往返、拒绝路径的运行输入
  chart/specs/               # plotter.chart：Vega-Lite 正例
  chart/rejections/          # plotter.chart：拒绝样例，逐条带 code 与 spanText
  notedown/                  # notedown 迁入的 5 组 lexer 语料 + 快照 + manifest.json
  acorn/doc/legacy_plain.doc # acorn 遗留 .doc 二进制语料
expectations/
  convert/report/            # document.convert：每个 case 一条断言清单
  ir/cases.json              # document.ir：接受/拒绝规则与 issue 码
  plotter/diagram/cases.json # plotter.diagram 与 DOT / Mermaid / PlantUML：正例与拒绝
  plotter/runtime/passes.json# plotter.runtime：动作到失效步骤
```

边界口径：

- `document.formats` 的期望直接写在 `fixtures/formats/manifest.json` 的 `expect` 里，因为它描述的是单一样例的导入结果。
- `document.convert` 的期望独立成文件，放在 `expectations/convert/report/`，与 `manifest.json` 的 `expectation` 字段一一对应。
- `plotter.chart` 的正例与拒绝都在 `fixtures/chart/`，拒绝行只引用共享 catalog 的 `code` 与 `spanText`。
- `plotter` 其余 crate 的期望在 `expectations/plotter/`，DOT / Mermaid / PlantUML 按活跃输入写成 `accept`，不要写成拒绝。
- 任何 `expectations/` 路径都经 `resolveExpectation` 校验，禁止绝对路径与 `..`。

## 公开读取面

- `fixturesRoot` / `expectationsRoot`：两个根目录绝对路径
- `resolveFixture(relativePath)` / `resolveExpectation(relativePath)`：路径穿越校验后的绝对路径
- `readFixtureBytes` / `readFixtureText` / `readFixtureJson` / `readExpectationJson`
- `loadNotedownManifest()`：解析 `notedown/manifest.json`，逐条返回语料与快照的绝对路径
- `loadFormatManifest()` / `loadChartCases()`：加载并校验 formats 与 chart 清单
- `loadConvertManifest()`：加载 `convert/manifest.json` 并校验每条输入存在
- `loadIrCases()` / `loadDiagramCases()` / `loadRuntimePasses()`：加载并校验期望数据

## 约束

- 不在其他仓或 `document.rs` 新建 fixture 目录。
- 不复制语料；需要引用时通过本包读取。
- 上游 Rust 测试代码不作为语料搬入，只保留真实输入与期望数据。`tests/upstream/*` 的 `.rs` 一行都不进本仓。
- 新增案例先加语料，再加期望，最后才在 harness 里登记可执行入口；没有执行入口的轴记为 gap，不伪造通过。
