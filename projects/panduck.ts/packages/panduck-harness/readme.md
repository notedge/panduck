# @notedge/panduck-harness

数据驱动的一致性 harness。语料来自 `@notedge/panduck-fixtures`，实现只通过 `@notedge/panduck` 的公开 Node-API 入口消费。

## 循环

```text
run -> archive -> classify -> tighten -> run
```

- `run`：逐 case 装载语料，调用公开入口，把观测归一化为 `text` / `html` / `exit` / `losses` / `diagnostics`
- `archive`：写入 `results/<runId>/{report.json,summary.md,observations/}`，并追加 `results/history.jsonl`
- `classify`：每个失败归入 `implementation-defect` / `stale-expectation` / `undetermined`
- `tighten`：`accept` 只允许追加断言，写入 `author` / `reason` / `semantic` / `updatedAt`

## 断言强度

| 轴 | 强度 | 当前状态 |
|---|---|---|
| `snapshot-equality` | 严格等值 | 阻塞：公开入口没有 lexer token 流 |
| `oracle-summary` | 第三方摘要级 | 阻塞：manifest 记录了 oracle，但没有可调用的摘要入口 |
| corpus integrity | 严格 | 快照形状、计数、offset 单调性 |
| conversion invariant | 语义不变量 | 入口存在即执行 |
| declared rejection | 精确诊断码 | 入口存在即执行 |

第三方渲染差异不会被当作实现 bug。只有公开路由自己声明的拒绝路径才写拒绝期望。

## 命令

```powershell
pnpm --filter @notedge/panduck-fixtures build
pnpm --filter @notedge/panduck-harness build
pnpm --filter @notedge/panduck-harness run
pnpm --filter @notedge/panduck-harness run:conversion
pnpm --filter @notedge/panduck-harness run:notedown
pnpm --filter @notedge/panduck-harness baseline -- --name first
pnpm --filter @notedge/panduck-harness accept -- --case diagram-render --add-file <json> --by <name> --why <reason> --semantic <semantic>
```

`accept` 会先重跑该 case，新增断言必须当场通过才会写入。它不能删除、替换或放宽既有断言。

## 失败处置

- `implementation-defect`：修实现。Rust 侧不在本 harness 的改动范围，登记后回报
- `stale-expectation`：人工确认语义变化后，显式修改期望并补 provenance，不做自动刷新
- `undetermined`：保持失败，登记到 `report.gaps`

## 目录

```text
bin/cli.mjs
expectations/convert/report/*.json
baselines/
src/{adapter,assertions,archive,accept,corpus,run,main,types,util}.ts
results/                 # 运行期归档，gitignored
```

case 目录来自 `@notedge/panduck-fixtures` 的 `fixtures/notedown/manifest.json` 与 `fixtures/convert/manifest.json`，harness 不自带第二份 case 清单。
