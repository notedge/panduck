# 示例落点

`examples/*` 只演示「下游怎么消费 Panduck」，不放语料、不放期望数据。

- 语料与期望归 `../packages/panduck-fixtures`：输入在 `fixtures/`，期望在 `expectations/`
- 执行器归 `../packages/panduck-harness`：`verify` 只验语料结构，`run` 才调 binding
- 示例自己带最小输入，不引用 `fixtures/` 里的样例，避免示例和语料耦合

当前示例：

| 目录 / 脚本 | 演示什么 | 命令 |
| --- | --- | --- |
| `convert-markdown/` | 自带输入的 markdown 转 html | `pnpm --filter @notedge/panduck-examples convert-markdown` |
| `conformance/` | 调 harness 的 `verify`，检查语料可被下游读取 | `pnpm --filter @notedge/panduck-examples conformance` |
| `markdown-to-html.mjs` | 语料里的 DOT / Mermaid / PlantUML 围栏渲染成 SVG | `pnpm --filter @notedge/panduck-examples markdown-to-html` |
| `roundtrip-docx.mjs` | markdown 经 docx 往返 | `pnpm --filter @notedge/panduck-examples roundtrip-docx` |
| `pdf-route.mjs` | markdown 经 pdf 往返 | `pnpm --filter @notedge/panduck-examples pdf-route` |
| `html-refusal.mjs` | 行内属性的拒绝路径与诊断码 | `pnpm --filter @notedge/panduck-examples html-refusal` |
