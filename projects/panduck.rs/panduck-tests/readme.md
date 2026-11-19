# panduck-tests

`document.*` 功能验收套件（自 rust-boost 迁入）。Boost 的 document crate 只保留冒烟测试；
转换报告、docx/epub/html/pdf/tex/typst/xlsx/pptx、pandoc 对照与发布流程在这里执行。

- `tests/convert.rs` — document.convert 全功能套件（148 例）。
- `tests/formats.rs` — document.formats 全功能套件（558 例）。
- `tests/container_import.rs` — 容器导入（10 例）。
- 共享验收件来自 Boost `document-harness`（报告断言、OOXML zip 构造器、字体包解析）。

PDF 写出用例需要宿主字体包：设置 `VUTEX_FONT_DIR` 指向 VuTeX 的 `fonts` 目录，
或从包含 `VuTeX/projects/vutex.rs/fonts` 的祖先目录运行。缺包时这些用例失败而不是跳过
（PDF 写出合同要求）。

依赖按 rust-boost git rev 固定；Boost 侧 document 家族有行为变更时，先升 pin 再跑本套件。
