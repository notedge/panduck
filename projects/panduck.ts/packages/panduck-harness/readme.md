# @notedge/panduck-harness

数据驱动的一致性 harness。语料来自 `@notedge/panduck-fixtures`，实现只通过 `@notedge/panduck` 的公开 Node-API 入口消费。

## 循环

```text
run -> archive -> classify -> tighten -> run
```

- `run`：逐 case 装载语料，调用公开入口，把观测归一化为 `text` / `html` / `exit` / `losses` / `diagnostics`
- `archive`：写入 `results/<runId>/{report.json,summary.md,observations/}`，并追加 `results/history.jsonl`
- `classify`：每个失败归入 `implementation-defect` / `stale-expectation` / `undetermined`
- `tighten`：`accept` 只允许追加断言，作者自动取 `<git user.name> <git user.email> @ <HEAD sha>`，每次追加一条 provenance，含断言 diff 与观测状态
- `oracle`：把已确认的缺陷固化成复现断言，允许当场为红，但同样记 diff 与观测状态，修好后复跑转绿再改分类

## 断言强度

| 轴 | 强度 | 当前状态 |
|---|---|---|
| `snapshot-equality` | 严格等值 | 阻塞：公开入口没有 lexer token 流 |
| `oracle-summary` | 第三方摘要级 | 未接入：如需要，在 harness 内直接依赖 `pulldown-cmark` / `markdown-rs` / `html5ever` 的 npm 包，不进产品 API |
| corpus integrity | 严格 | 快照形状、计数、offset 单调性 |
| conversion invariant | 语义不变量 | 入口存在即执行 |
| declared rejection | 精确诊断码 | 入口存在即执行 |

第三方渲染差异不会被当作实现 bug。只有公开路由自己声明的拒绝路径才写拒绝期望。

## 命令

```powershell
pnpm --filter @notedge/panduck-fixtures build
pnpm --filter @notedge/panduck-harness build
node projects/panduck.ts/packages/panduck-harness/bin/cli.mjs run
node projects/panduck.ts/packages/panduck-harness/bin/cli.mjs run --suite convert
node projects/panduck.ts/packages/panduck-harness/bin/cli.mjs run --suite notedown
node projects/panduck.ts/packages/panduck-harness/bin/cli.mjs verify
node projects/panduck.ts/packages/panduck-harness/bin/cli.mjs baseline --name first
node projects/panduck.ts/packages/panduck-harness/bin/cli.mjs accept --case diagram-render --add-file <json> --why <reason> --semantic <semantic>
node projects/panduck.ts/packages/panduck-harness/bin/cli.mjs oracle --case basic-markdown-to-html --add '<json>' --why <reason> --semantic <semantic>
```

`accept` 会先重跑该 case，新增断言必须当场通过才会写入，它不能删除、替换或放宽既有断言。`oracle` 用于已知缺陷的复现断言，允许为红，但不会把用例改判为通过，也不会删掉既有断言。

## 失败处置

- `implementation-defect`：修实现。Rust 侧不在本 harness 的改动范围，登记后回报
- `stale-expectation`：人工确认语义变化后，显式修改期望并补 provenance，不做自动刷新
- `undetermined`：保持失败，登记到 `report.gaps`

## 目录

```text
bin/cli.mjs
baselines/
src/{adapter,assertions,archive,accept,corpus,main,types,util}.ts
results/                 # 运行期归档，gitignored
```

期望文件在 `@notedge/panduck-fixtures` 的 `expectations/convert/report/*.json`，harness 不自带第二份。

case 目录来自 `@notedge/panduck-fixtures` 的 `fixtures/notedown/manifest.json` 与 `fixtures/convert/manifest.json`，harness 不自带第二份 case 清单。
