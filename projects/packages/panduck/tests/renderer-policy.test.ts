import assert from "node:assert/strict";
import test from "node:test";

import { Converter } from "../src/converter.js";
import { defineConversion } from "../src/conversion-plan.js";
import { defineMarkdownPolicy, withHandlers } from "../src/renderer-policy.js";
import type { ConverterConfig } from "../src/renderer-policy.js";

function sampleHtml(body: string): string {
    return `<!doctype html><html><head></head><body>${body}</body></html>`;
}

// ---------------------------------------------------------------------------
// Policy 构造函数 —— 纯类型层，无需 Rust
// ---------------------------------------------------------------------------

test("defineMarkdownPolicy returns gfm-default with overrides merged", () => {
    const p = defineMarkdownPolicy();
    assert.equal(p.name, "gfm-default");
    assert.equal(p.heading, "atx");
    assert.equal(p.bullet, "-");
    assert.equal(p.fencedCode, true);
    assert.equal(p.unsupported, "report");
    assert.deepEqual(p.handlers, {});

    const q = defineMarkdownPolicy({ name: "custom", heading: "setext" });
    assert.equal(q.name, "custom");
    assert.equal(q.heading, "setext");
    // 未覆盖的字段仍来自默认
    assert.equal(q.bullet, "-");
});

test("withHandlers merges handlers without mutating source policy", () => {
    const base = defineMarkdownPolicy();
    const withA = withHandlers(base, { highlight: () => ({ kind: "fragment", markdown: "A" }) });
    const withB = withHandlers(withA, { suppress: () => ({ kind: "suppress" }) });

    assert.equal(Object.keys(base.handlers ?? {}).length, 0, "source policy must be untouched");
    assert.equal(Object.keys(withA.handlers ?? {}).length, 1);
    assert.equal(Object.keys(withB.handlers ?? {}).length, 2);
    assert.ok(withB.handlers?.highlight);
    assert.ok(withB.handlers?.suppress);
});

test("PolicyHandler 返回值三分支合法", () => {
    let sawPass = false;
    let sawFrag = false;
    let sawSupp = false;

    const policy = defineMarkdownPolicy({
        name: "branch-check",
        handlers: {
            pass: () => { sawPass = true; return { kind: "pass-through" }; },
            frag: () => { sawFrag = true; return { kind: "fragment", markdown: "CUSTOM" }; },
            supp: () => { sawSupp = true; return { kind: "suppress" }; },
        },
    });

    // 这是纯类型 + 逻辑测试：handlers 本身合法，返回值符合三分支。
    assert.ok(policy.handlers);
    assert.equal(typeof policy.handlers.pass, "function");
    assert.equal(typeof policy.handlers.frag, "function");
    assert.equal(typeof policy.handlers.supp, "function");

    policy.handlers.pass({ blockKind: "pass" });
    policy.handlers.frag({ blockKind: "frag" });
    policy.handlers.supp({ blockKind: "supp" });

    assert.ok(sawPass);
    assert.ok(sawFrag);
    assert.ok(sawSupp);
});

// ---------------------------------------------------------------------------
// ConverterConfig 进入 Converter —— policy-active 诊断
// ---------------------------------------------------------------------------

test("toMarkdown with ConverterConfig uses policy path and emits policy-active diagnostic", async () => {
    const converter = new Converter();

    const config: ConverterConfig = {
        rendererPolicy: defineMarkdownPolicy({ name: "test-policy" }),
    };

    // 简单 HTML（包含 HTML 注释）
    const html = sampleHtml(`<p>hello world</p>`);
    const result = await converter.toMarkdown(html, config);

    // 必须包含 policy-active 诊断
    assert.ok(
        result.diagnostics.some((d) => d.code === "panduck.convert.policy-active"),
        `Expected policy-active diagnostic, got codes: ${result.diagnostics.map((d) => d.code).join(", ")}`,
    );

    // markdown 输出非空（Rust 转换成功）
    assert.ok(result.text.length > 0, "Markdown output must be non-empty");
    assert.ok(result.text.includes("hello world"), `Markdown must contain original text. Got: ${result.text}`);
});

test("toMarkdown: legacy ToMarkdownOptions path still works (backward compat)", async () => {
    const converter = new Converter();
    const result = await converter.toMarkdown(
        sampleHtml(`<p>hello legacy</p>`),
        { from: "html", label: "legacy-test" },
    );

    assert.ok(result.text.includes("hello legacy"), `Legacy path must convert. Got: ${result.text}`);
    // 不能有 policy-active 诊断（没带 ConverterConfig）
    assert.ok(
        !result.diagnostics.some((d) => d.code === "panduck.convert.policy-active"),
        "Legacy path must NOT emit policy-active diagnostic",
    );
});

// ---------------------------------------------------------------------------
// Policy handlers 只是声明面 —— 不在 TS 侧执行，只产生 Warning
// ---------------------------------------------------------------------------

test("policy handlers warn as unsupported and are never executed", async () => {
    const converter = new Converter();

    let invoked = 0;
    const policy = defineMarkdownPolicy({
        name: "rewrite-test",
        handlers: {
            "custom.important": () => {
                invoked++;
                return { kind: "fragment", markdown: "**[重写]" };
            },
        },
    });

    const config: ConverterConfig = { rendererPolicy: policy };

    const html = sampleHtml(
        `<p>regular paragraph</p>` +
        `<div><!-- panduck-policy:custom.important -->this should stay</div>` +
        `<p>another paragraph</p>`,
    );

    const result = await converter.toMarkdown(html, config);

    assert.ok(
        result.diagnostics.some((d) => d.code === "panduck.convert.policy-active"),
        "policy-active diagnostic should be present",
    );

    const warning = result.diagnostics.find(
        (d) => d.code === "panduck.policy.handlers-unsupported",
    );
    assert.ok(
        warning,
        `Expected handlers-unsupported warning, got codes: ${result.diagnostics.map((d) => d.code).join(", ")}`,
    );
    assert.equal(warning.severity, "warn");
    assert.deepEqual(warning.args?.handlerKeys, ["custom.important"]);
    assert.equal(warning.args?.policyName, "rewrite-test");

    assert.equal(invoked, 0, "policy handlers must not be executed in the TS layer");
    assert.ok(result.text.includes("this should stay"), `Original text must survive. Got: ${result.text}`);
    assert.ok(!result.text.includes("**[重写]"), "No handler fragment may be synthesized");
    assert.notEqual(result.status, "failed");
});

test("policy without handlers emits no handlers-unsupported warning", async () => {
    const converter = new Converter();
    const config: ConverterConfig = {
        rendererPolicy: defineMarkdownPolicy({ name: "plain-policy" }),
    };

    const result = await converter.toMarkdown(sampleHtml(`<p>plain body</p>`), config);

    assert.ok(
        !result.diagnostics.some((d) => d.code === "panduck.policy.handlers-unsupported"),
        "handlers-unsupported must only fire when handlers are declared",
    );
});

test("ConversionPlan output policy handlers warn instead of claiming execution", async () => {
    const converter = new Converter();

    let invoked = 0;
    const plan = defineConversion({
        id: "handlers-plan-check",
        input: { format: "html" },
        output: {
            format: "markdown",
            policy: withHandlers(defineMarkdownPolicy({ name: "plan-policy" }), {
                "custom.ads": () => {
                    invoked++;
                    return { kind: "suppress" };
                },
            }),
        },
    });

    const result = await converter.toMarkdown(
        sampleHtml(`<p>keep me</p><p>buy now!</p>`),
        plan,
    );

    const warning = result.diagnostics.find(
        (d) => d.code === "panduck.policy.handlers-unsupported",
    );
    assert.ok(
        warning,
        `Expected handlers-unsupported warning, got codes: ${result.diagnostics.map((d) => d.code).join(", ")}`,
    );
    assert.equal(warning.severity, "warn");
    assert.deepEqual(warning.args?.handlerKeys, ["custom.ads"]);
    assert.equal(invoked, 0, "plan policy handlers must not be executed");
    assert.ok(result.text.includes("buy now"), `Original paragraph must survive. Got: ${result.text}`);
    assert.notEqual(result.status, "failed");
});
