//! 乐谱与化学结构在 HTML 导出中的真实渲染：`abc` 与 `smiles` 走 Oak 前端并嵌入
//! 内联 SVG，`musicxml` 与 `rd` 仍原样保留源 fence 并记录 `ast-unavailable`，
//! 不出现空白洞，也不臆造几何。

use document_convert::convert_bytes;

use crate::common;
use common::assert_convert_report;

const ABC: &str = "X:1\nT:Smoke\nM:4/4\nL:1/4\nK:C\nCDEF|";
const SMILES: &str = "CCO";

fn doc(language: &str, source: &str) -> String {
    format!("# Asset\n\n```{language}\n{source}\n```\n")
}

fn html_of(language: &str, source: &str) -> document_convert::ConvertOutput {
    convert_bytes("markdown", "html", "asset.md", doc(language, source).into_bytes(), None).expect("convert asset markdown")
}

fn report_codes(report_json: &str) -> Vec<String> {
    let report: serde_json::Value = serde_json::from_str(report_json).expect("parse report");
    report["losses"]
        .as_array()
        .map(|losses| losses.iter().filter_map(|loss| loss["code"].as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

#[test]
fn abc_fence_renders_an_inline_score_scene() {
    let output = html_of("abc", ABC);
    let html = output.html.as_deref().expect("html output");

    assert!(html.contains(r#"<figure class="engraving" data-engraving-language="abc""#), "{html}");
    assert!(html.contains("<svg"), "the score is a real vector scene: {html}");
    assert!(html.contains("engraving-"), "the scene ids carry the engraving namespace: {html}");

    let codes = report_codes(&output.report_json);
    assert!(!codes.iter().any(|code| code == "davinci.engraving.ast-unavailable"), "{:?}", codes);
    assert_convert_report(&output, "html");
}

#[test]
fn smiles_fence_renders_an_inline_structure_scene() {
    let output = html_of("smiles", SMILES);
    let html = output.html.as_deref().expect("html output");

    assert!(html.contains(r#"<figure class="depiction" data-depiction-language="smiles""#), "{html}");
    assert!(html.contains("<svg"), "the structure is a real vector scene: {html}");
    assert!(html.contains("depiction-"), "the scene ids carry the depiction namespace: {html}");

    let codes = report_codes(&output.report_json);
    assert!(!codes.iter().any(|code| code == "davinci.depiction.ast-unavailable"), "{:?}", codes);
    assert_convert_report(&output, "html");
}

#[test]
fn musicxml_fence_keeps_its_source_and_names_the_missing_frontend() {
    let output = html_of("musicxml", "<score-partwise/>");
    let html = output.html.as_deref().expect("html output");

    assert!(html.contains(r#"<pre class="engraving" data-engraving-language="musicxml""#), "{html}");
    assert!(!html.contains("<figure"), "no scene is invented without a frontend: {html}");

    let codes = report_codes(&output.report_json);
    assert!(codes.iter().any(|code| code == "davinci.engraving.ast-unavailable"), "{:?}", codes);
    assert_convert_report(&output, "html");
}

#[test]
fn rd_fence_keeps_its_source_and_names_the_missing_frontend() {
    let output = html_of("rd", SMILES);
    let html = output.html.as_deref().expect("html output");

    assert!(html.contains(r#"<pre class="depiction" data-depiction-language="rd""#), "{html}");

    let codes = report_codes(&output.report_json);
    assert!(codes.iter().any(|code| code == "davinci.depiction.ast-unavailable"), "{:?}", codes);
    assert_convert_report(&output, "html");
}

#[test]
fn non_registered_fence_language_stays_a_code_block() {
    let output = html_of("bogus", ABC);
    let html = output.html.as_deref().expect("html output");
    assert!(!html.contains("data-engraving-language"), "{html}");
    assert!(!html.contains("data-depiction-language"), "{html}");
    let codes = report_codes(&output.report_json);
    assert!(!codes.iter().any(|code| code.starts_with("davinci.engraving")), "{:?}", codes);
    assert!(!codes.iter().any(|code| code.starts_with("davinci.depiction")), "{:?}", codes);
}
