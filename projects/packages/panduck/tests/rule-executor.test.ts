import assert from "node:assert/strict";
import test from "node:test";

import { RuleBuilder } from "../src/rule-builder.js";
import { executePlan } from "../src/rule-executor.js";

function sampleHtml(body: string): string {
    return `<!doctype html><html><head></head><body>${body}</body></html>`;
}

// ---------------------------------------------------------------------------
// resolveRules equivalent — the bug we're fixing also affects this package's
// direct RulePlan consumers; we verify the handler round-trip here.
// ---------------------------------------------------------------------------

test("executePlan: replace handler IS invoked during execution and mutates DOM", () => {
    let invoked = 0;
    const builder = new RuleBuilder();
    builder.output((scope) => {
        scope.replace(".ad-banner", (_node, md) => {
            invoked++;
            return md.strong("广告已移除");
        });
    });

    const plan = builder.build();
    assert.equal(Object.keys(plan.handlers).length, 1, "RulePlan should carry the handler");

    const html = sampleHtml(`
        <div class="ad-banner">Buy now!</div>
        <p>Keep me</p>
    `);

    const result = executePlan(html, plan, plan.handlers);

    assert.equal(invoked, 1, "Handler should have been called exactly once");
    assert.ok(result.html.includes("**广告已移除**"), `Replacement markdown should appear in the DOM. Got: ${result.html}`);
    assert.ok(result.html.includes("Keep me"), "Non-matching content should survive");
    assert.equal(result.requireNoLoss, false);
    assert.ok(result.diagnostics.some((d) => d.code === "panduck.executor.replace-executed"));
});

test("executePlan: replace handler runs for every matched node", () => {
    let invoked = 0;
    const builder = new RuleBuilder();
    builder.output((scope) => {
        scope.replace("em", () => {
            invoked++;
            return "[italic]";
        });
    });

    const plan = builder.build();
    const html = sampleHtml(`
        <p><em>a</em> <em>b</em> <em>c</em></p>
    `);

    const result = executePlan(html, plan, plan.handlers);
    assert.equal(invoked, 3, "Handler should fire three times for three matches");
    // Each <em> is replaced, so result should contain three [italic] wrappers.
    const matches = result.html.match(/\[italic\]/g);
    assert.ok(matches, `Expected three [italic] markers. HTML: ${result.html}`);
    assert.equal(matches!.length, 3);
});

test("executePlan: handler missing from registry surfaces error diagnostic and is skipped", () => {
    const builder = new RuleBuilder();
    builder.output((scope) => {
        scope.replace(".ghost", () => "NEVER SEEN");
    });
    const plan = builder.build();

    // Pass an EMPTY handler registry — simulates the RulePlan.handlers drop bug.
    const result = executePlan(sampleHtml(`<div class="ghost">hi</div>`), plan, {});
    assert.ok(result.diagnostics.some((d) => d.code === "panduck.executor.replace-handler-missing"));
    // Original DOM should still be there (no mutation) because handler was not found.
    assert.ok(result.html.includes('class="ghost"'));
    assert.ok(!result.html.includes("NEVER SEEN"));
});

test("executePlan: handler selector no-match surfaces warn diagnostic", () => {
    let invoked = 0;
    const builder = new RuleBuilder();
    builder.output((scope) => {
        scope.replace("does-not-exist", () => {
            invoked++;
            return "nope";
        });
    });
    const plan = builder.build();
    const result = executePlan(sampleHtml(`<p>only text</p>`), plan, plan.handlers);
    assert.equal(invoked, 0);
    assert.ok(result.diagnostics.some((d) => d.code === "panduck.executor.replace-no-match"));
});

test("executePlan: handler throwing surfaces error diagnostic and survives execution", () => {
    const thrower = () => { throw new Error("boom"); };
    const builder = new RuleBuilder();
    builder.output((scope) => {
        scope.replace("span", thrower);
    });
    const plan = builder.build();
    const result = executePlan(sampleHtml(`<span>x</span>`), plan, plan.handlers);
    assert.ok(result.diagnostics.some((d) => d.code === "panduck.executor.replace-handler-threw"));
    // Original DOM untouched.
    assert.ok(result.html.includes("<span>x</span>"));
});

test("executePlan: requireNoLoss flag propagates through result", () => {
    const builder = new RuleBuilder();
    builder.output((scope) => { scope.requireNoLoss(); });
    const plan = builder.build();
    const result = executePlan(sampleHtml("<p>ok</p>"), plan, plan.handlers);
    assert.equal(result.requireNoLoss, true);
});

test("executePlan: output.replace after transform sees post-transform DOM", () => {
    let invoked = 0;
    const builder = new RuleBuilder();
    builder
        .transform((scope) => scope.unwrap("section.inner"))
        .output((scope) => scope.replace(".target", () => { invoked++; return "got-it"; }));

    const plan = builder.build();
    const html = sampleHtml(`
        <section class="inner"><p class="target">hi</p></section>
        <p class="target">outer</p>
    `);
    const result = executePlan(html, plan, plan.handlers);
    // After unwrap, both .target p's should be visible → handler runs twice.
    assert.equal(invoked, 2);
    const hits = result.html.match(/got-it/g);
    assert.equal(hits?.length ?? 0, 2);
});
