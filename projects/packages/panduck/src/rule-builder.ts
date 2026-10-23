/**
 * Panduck RuleBuilder — a declarative, domain-scoped plan builder for
 * document manipulation rules.
 *
 * @deprecated 用 {@link defineConversion} 替代。
 *
 * RuleBuilder 面向 HTML DOM（linkedom CSS selector + DOM mutation），
 * 是 Panduck 在 Notedown IR 还没就绪前的过渡方案。ConversionPlan 面向
 * notedown-ir DocumentGraph，是正式的转换合同。当前 RuleBuilder 继续
 * 可用但不接受新功能；等 IR transform pass 在 Rust 侧落地后删除。
 *
 * Contract (per design doc):
 *   - `input`     — read / select / assert structure over the current scope.
 *   - `transform` — mutate the working document; subsequent `input` /
 *                   `transform` see the result.
 *   - `output`    — register target-format rendering rules. Does NOT close
 *                   the plan; more `input` or `transform` can follow.
 *                   CRITICAL: `.output.replace()` DEFERRED — registers handler
 *                   key only; NO DOM mutation, NO handler execution here.
 *
 * The three domains are **not** a state machine — they can appear in any
 * order and any number of times. Each domain callback produces one RuleStep
 * in the final RulePlan; order is preserved exactly as declared.
 */

// ---------------------------------------------------------------------------
// Plan — the executable deliverable from `.build()`
// ---------------------------------------------------------------------------

export type RuleDomain = "input" | "transform" | "output";

// Input ops — select scope or assert required structure.
export type InputOp =
    | { readonly op: "select"; readonly selector: string }
    | { readonly op: "require"; readonly key: string; readonly selector: string };

// Transform ops — mutate the working document.
export type TransformOp =
    | { readonly op: "remove"; readonly selector: string }
    | { readonly op: "unwrap"; readonly selector: string };

// Output ops — register rendering rules and delivery constraints.
export type ReplaceHandlerFn = (
    node: unknown,
    md: MarkdownHelper,
) => string;

export type MarkdownHelper = {
    children: (node: unknown) => string;
    quote: (text: string) => string;
    emphasis: (text: string) => string;
    strong: (text: string) => string;
};

export type OutputOp =
    | {
          readonly op: "replace";
          readonly selector: string;
          readonly handlerKey: string;
      }
    | { readonly op: "require-no-loss" };

export type RuleOp = InputOp | TransformOp | OutputOp;

/** One domain step — each `.input(...)` / `.transform(...)` / `.output(...)` creates exactly one. */
export type RuleStep = {
    readonly domain: RuleDomain;
    readonly ops: readonly RuleOp[];
};

/**
 * Full executable RulePlan — the `.build()` product.
 *
 * @deprecated 用 {@link ConversionPlan} 替代。
 *
 * v0 contract: `.build()` returns **everything needed to execute** this plan.
 * Handlers (closures) are NOT JSON-serializable but ARE carried intact —
 * Converter receives this shape and reads handlers from it directly.
 *
 * For a JSON-serializable description view, call `.buildDescriptor()`.
 *
 * **NO two shapes.** `.build()` is the only execution entry.
 */
export type RulePlan = {
    readonly version: "panduck-rule-v0";
    readonly steps: readonly RuleStep[];
    /** Full handler bindings — closures, not serializable. Rust dispatch host consumes these. */
    readonly handlers: Record<string, ReplaceHandlerFn>;
};

/**
 * Pure JSON-serializable RuleDescriptor — for persistence / cross-process
 * transmission where closures cannot survive. Separate from `.build()`.
 */
export type RuleDescriptor = {
    readonly version: "panduck-rule-v0";
    readonly steps: readonly RuleStep[];
    handlerKeys: readonly string[];  // Which handlers need to be provided at execute-time
};

// ---------------------------------------------------------------------------
// Scope classes — TS type gates for what's legal in each domain
// ---------------------------------------------------------------------------

/**
 * Read / select / assert structure. No mutation, no output rule registration.
 * @deprecated RuleBuilder 整体 deprecated — 用 {@link defineConversion}。
 */
export class InputScope {
    private readonly _ops: InputOp[] = [];
    get ops(): readonly InputOp[] { return this._ops; }

    select(selector: string): this {
        this._ops.push({ op: "select", selector });
        return this;
    }

    require(key: string, selector: string): this {
        this._ops.push({ op: "require", key, selector });
        return this;
    }
}

/**
 * Mutate the working document — effects visible to subsequent steps.
 * @deprecated RuleBuilder 整体 deprecated — 用 {@link defineConversion}。
 */
export class TransformScope {
    private readonly _ops: TransformOp[] = [];
    get ops(): readonly TransformOp[] { return this._ops; }

    remove(selector: string): this {
        this._ops.push({ op: "remove", selector });
        return this;
    }

    unwrap(selector: string): this {
        this._ops.push({ op: "unwrap", selector });
        return this;
    }
}

/**
 * Register target-format output rules.
 *
 * @deprecated RuleBuilder 整体 deprecated — 用 {@link defineConversion}。
 *
 * `.replace()` records selector + handlerKey and registers the handler on
 * the owning RuleBuilder. **Does NOT execute, does NOT modify DOM, does NOT
 * embed sentinels.** Handler execution is deferred to the Rust Notedown/Oak
 * lowering + dispatch host.
 */
export class OutputScope {
    private readonly _ops: OutputOp[] = [];
    constructor(private readonly _builder: RuleBuilder) {}

    get ops(): readonly OutputOp[] { return this._ops; }

    replace(selector: string, handler: ReplaceHandlerFn): this {
        const key = this._builder.registerHandler(handler);
        this._ops.push({ op: "replace", selector, handlerKey: key });
        return this;
    }

    requireNoLoss(): this {
        this._ops.push({ op: "require-no-loss" });
        return this;
    }
}

// ---------------------------------------------------------------------------
// RuleBuilder — the public entry
// ---------------------------------------------------------------------------

/**
 * Panduck RuleBuilder — 声明式 HTML DOM 级规则构建器。
 *
 * @deprecated 用 {@link defineConversion} 替代。
 *
 * RuleBuilder 面向 linkedom CSS selector + DOM mutation，是 Panduck 早期
 * 在 Notedown IR 还没就绪前的过渡方案。ConversionPlan 面向 notedown-ir
 * DocumentGraph，是正式的转换合同。当前 RuleBuilder 继续可用但不接受
 * 新功能；等 IR transform pass 在 Rust 侧落地后删除。
 */
export class RuleBuilder {
    private readonly _steps: RuleStep[] = [];
    /** Collected ReplaceHandlerFn keyed by their stable opaque key. */
    private readonly _handlers: Record<string, ReplaceHandlerFn> = {};
    /** Per-builder anonymous counter — not module-global, so multiple builders don't collide. */
    private _anonCounter = 0;

    /**
     * Register a custom handler. Returns the stable key.
     * - Named function: key = handler.name (if empty → falls through to anon)
     * - Anonymous: key = `anon-${builderId}-${_anonCounter++}`
     */
    registerHandler(handler: ReplaceHandlerFn): string {
        const name = handler.name?.trim();
        if (name && !this._handlers[name]) {
            this._handlers[name] = handler;
            return name;
        }
        // Anonymous or duplicate name — use per-builder anon counter
        const key = `anon-${this._anonCounter++}`;
        this._handlers[key] = handler;
        return key;
    }

    /** Expose handlers for executor consumption — v0 contract, no compat shim. */
    get handlers(): Record<string, ReplaceHandlerFn> {
        return this._handlers;
    }

    /** Read / select / assert structure. */
    input(cb: (scope: InputScope) => void): this {
        const scope = new InputScope();
        cb(scope);
        if (scope.ops.length > 0) {
            this._steps.push({ domain: "input", ops: [...scope.ops] });
        }
        return this;
    }

    /** Mutate the working document — effects visible to subsequent steps. */
    transform(cb: (scope: TransformScope) => void): this {
        const scope = new TransformScope();
        cb(scope);
        if (scope.ops.length > 0) {
            this._steps.push({ domain: "transform", ops: [...scope.ops] });
        }
        return this;
    }

    /** Register target-format rendering rules and delivery constraints. */
    output(cb: (scope: OutputScope) => void): this {
        const scope = new OutputScope(this);
        cb(scope);
        if (scope.ops.length > 0) {
            this._steps.push({ domain: "output", ops: [...scope.ops] });
        }
        return this;
    }

    /**
     * Produce the **full executable** RulePlan — steps + handlers intact.
     * This is the v0 execution entry. Converter receives this shape directly.
     */
    build(): RulePlan {
        return {
            version: "panduck-rule-v0",
            steps: [...this._steps],
            handlers: { ...this._handlers },
        };
    }

    /**
     * Produce a **pure JSON-serializable** RuleDescriptor — steps + handlerKeys.
     * Handlers themselves (closures) cannot cross boundaries; the descriptor
     * tells the execution host which handlers it needs to supply.
     */
    buildDescriptor(): RuleDescriptor {
        const handlerKeys = Object.keys(this._handlers);
        return {
            version: "panduck-rule-v0",
            steps: [...this._steps],
            handlerKeys,
        };
    }
}
