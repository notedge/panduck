/**
 * Paint analysis for one rendered diagram SVG.
 *
 * A non-empty `<svg>` says nothing about visibility: the DaVinci theme paints
 * shape roles through CSS class rules, an authored scene paints through
 * attributes, and a scene with no palette or role at all paints through
 * neither. The harness resolves both sources — including paint inherited from
 * an ancestor `<g>` and descendant selectors such as
 * `.panduck-diagram-edge polygon` — and counts geometry that actually carries a
 * visible fill or stroke, so an "accepted but blank" diagram fails a named
 * assertion instead of passing on string length.
 */

/** Shape elements that can carry paint. Text is measured but never counted. */
const GEOMETRY_TAGS = new Set(["path", "rect", "circle", "ellipse", "polygon", "polyline", "line"]);

/** Attribute values that mean "no paint". */
const EMPTY_PAINT = new Set(["none", "transparent", ""]);

/** Paint evidence for one rendered SVG. */
export type SvgPaintEvidence = {
    /** Shape elements written into the SVG. */
    readonly elements: number;
    /** Text elements written into the SVG. */
    readonly texts: number;
    /** Shape elements that carry a visible fill or stroke. */
    readonly painted: number;
    /** Painted through an attribute on the element itself. */
    readonly attributePainted: number;
    /** Painted through a matching class rule, directly or inherited. */
    readonly classPainted: number;
    /** Distinct visible fill values. */
    readonly fills: readonly string[];
    /** Distinct visible stroke values. */
    readonly strokes: readonly string[];
    /** Shape elements whose resolved paint is fully transparent. */
    readonly erased: number;
};

const TAG = /<\/?([a-zA-Z][a-zA-Z0-9-]*)((?:[^>"']|"[^"]*"|'[^']*')*)>/g;
const ATTRIBUTE = /([a-zA-Z_:][a-zA-Z0-9_:.-]*)="([^"]*)"/g;
const STYLE_BLOCK = /<style[^>]*>([\s\S]*?)<\/style>/g;

type PaintValues = {
    fill: string | undefined;
    stroke: string | undefined;
    opacity: number | undefined;
    fillOpacity: number | undefined;
    strokeOpacity: number | undefined;
};

type PaintSource = "attribute" | "class" | "inherited" | "none";

type ResolvedPaint = PaintValues & {
    readonly visible: boolean;
    readonly fillSource: PaintSource;
    readonly strokeSource: PaintSource;
};

/** A `<style>` rule reduced to the compounds a chain of elements must match. */
type Compound = {
    readonly tag: string | undefined;
    readonly classes: readonly string[];
};

type ClassRule = {
    readonly compounds: readonly Compound[];
    readonly paint: Partial<Record<"fill" | "stroke" | "opacity" | "fillOpacity" | "strokeOpacity", string>>;
};

/** Split a declaration block into property/value pairs. */
function declarations(block: string): Map<string, string> {
    const entries = new Map<string, string>();
    for (const part of block.split(";")) {
        const index = part.indexOf(":");
        if (index <= 0) {
            continue;
        }
        entries.set(part.slice(0, index).trim(), part.slice(index + 1).trim());
    }
    return entries;
}

/** Reduce one compound selector such as `.a.b` or `polygon` to its matcher. */
function compound(text: string): Compound | undefined {
    const classes = [...text.matchAll(/\.([a-zA-Z0-9_-]+)/g)].map((match) => match[1]);
    const tag = text
        .replace(/#[a-zA-Z0-9_-]+/g, "")
        .replace(/\[[^\]]*\]/g, "")
        .replace(/:[a-zA-Z-]+(\([^)]*\))?/g, "")
        .match(/^([a-zA-Z][a-zA-Z0-9-]*)/)?.[1]
        ?.toLowerCase();
    if (classes.length === 0 && tag === undefined) {
        return undefined;
    }
    return { tag, classes };
}

/** Parse every class rule out of the SVG's `<style>` blocks. */
function classRules(svg: string): readonly ClassRule[] {
    const rules: ClassRule[] = [];
    for (const block of svg.matchAll(STYLE_BLOCK)) {
        const body = block[1].replace(/\/\*[\s\S]*?\*\//g, "");
        for (const rule of body.split("}")) {
            const open = rule.lastIndexOf("{");
            if (open <= 0) {
                continue;
            }
            // An at-rule prefix such as `@media (prefers-color-scheme: dark) {`
            // wraps the selector, so the selector starts after the last `{`.
            const selector = rule.slice(rule.lastIndexOf("{", open - 1) + 1, open).trim();
            for (const branch of selector.split(",")) {
                const compounds = branch
                    .trim()
                    .split(/\s+|\s*>\s*|\s*\+\s*/)
                    .map(compound)
                    .filter((part): part is Compound => part !== undefined);
                if (compounds.length === 0) {
                    continue;
                }
                const entries = declarations(rule.slice(open + 1));
                rules.push({
                    compounds,
                    paint: {
                        fill: entries.get("fill"),
                        stroke: entries.get("stroke"),
                        opacity: entries.get("opacity"),
                        fillOpacity: entries.get("fill-opacity"),
                        strokeOpacity: entries.get("stroke-opacity"),
                    },
                });
            }
        }
    }
    return rules;
}

function optionalNumber(value: string | undefined): number | undefined {
    const parsed = value === undefined ? Number.NaN : Number(value);
    return Number.isFinite(parsed) ? parsed : undefined;
}

/** One element as the parser sees it. */
type Element = {
    readonly name: string;
    readonly attributes: Map<string, string>;
    readonly classes: readonly string[];
    readonly selfClosing: boolean;
    readonly closing: boolean;
};

/** Walk the SVG once, yielding elements in document order. */
function* elements(svg: string): Generator<Element> {
    const body = svg.replace(STYLE_BLOCK, "");
    for (const tag of body.matchAll(TAG)) {
        const attributes = new Map<string, string>();
        for (const attribute of tag[2].matchAll(ATTRIBUTE)) {
            attributes.set(attribute[1], attribute[2]);
        }
        yield {
            name: tag[1].toLowerCase(),
            attributes,
            classes: attributes.get("class")?.split(/\s+/).filter(Boolean) ?? [],
            selfClosing: tag[2].trimEnd().endsWith("/"),
            closing: tag[0].startsWith("</"),
        };
    }
}

/** True when `element` is the subject of the rule's last compound. */
function matchesSubject(rule: ClassRule, element: Element): boolean {
    const subject = rule.compounds[rule.compounds.length - 1];
    if (subject.classes.some((name) => !element.classes.includes(name))) {
        return false;
    }
    return subject.tag === undefined || subject.tag === element.name;
}

/** True when every earlier compound is carried by one ancestor. */
function matchesAncestors(rule: ClassRule, ancestors: readonly Element[]): boolean {
    let cursor = 0;
    for (let index = 0; index < rule.compounds.length - 1; index += 1) {
        const wanted = rule.compounds[index];
        while (cursor < ancestors.length) {
            const candidate = ancestors[cursor];
            cursor += 1;
            if (
                wanted.classes.every((name) => candidate.classes.includes(name)) &&
                (wanted.tag === undefined || wanted.tag === candidate.name)
            ) {
                break;
            }
            if (cursor >= ancestors.length) {
                return false;
            }
        }
    }
    return true;
}

/** Paint an element inherits from its ancestors, before its own declarations. */
function inherited(ancestors: readonly Element[], chain: readonly ResolvedPaint[]): ResolvedPaint {
    for (let index = chain.length - 1; index >= 0; index -= 1) {
        const paint = chain[index];
        if (paint.fill !== undefined || paint.stroke !== undefined) {
            return { ...paint, fillSource: "inherited", strokeSource: "inherited" };
        }
    }
    return {
        fill: undefined,
        stroke: undefined,
        opacity: undefined,
        fillOpacity: undefined,
        strokeOpacity: undefined,
        visible: false,
        fillSource: "none",
        strokeSource: "none",
    };
}

/** Resolve one element against its ancestors and the theme rules. */
function resolve(
    element: Element,
    ancestors: readonly Element[],
    chain: readonly ResolvedPaint[],
    rules: readonly ClassRule[],
): ResolvedPaint {
    const base = inherited(ancestors, chain);
    let fill = base.fill;
    let stroke = base.stroke;
    let fillSource = base.fillSource;
    let strokeSource = base.strokeSource;
    let opacity = base.opacity ?? optionalNumber(element.attributes.get("opacity"));
    let fillOpacity = base.fillOpacity ?? optionalNumber(element.attributes.get("fill-opacity"));
    let strokeOpacity = base.strokeOpacity ?? optionalNumber(element.attributes.get("stroke-opacity"));
    const attributeFill = element.attributes.get("fill");
    const attributeStroke = element.attributes.get("stroke");
    if (attributeFill !== undefined) {
        fill = attributeFill;
        fillSource = "attribute";
    }
    if (attributeStroke !== undefined) {
        stroke = attributeStroke;
        strokeSource = "attribute";
    }

    for (const rule of rules) {
        if (!matchesSubject(rule, element) || !matchesAncestors(rule, ancestors)) {
            continue;
        }
        if (rule.paint.fill !== undefined) {
            fill = rule.paint.fill;
            fillSource = "class";
        }
        if (rule.paint.stroke !== undefined) {
            stroke = rule.paint.stroke;
            strokeSource = "class";
        }
        opacity ??= optionalNumber(rule.paint.opacity);
        fillOpacity ??= optionalNumber(rule.paint.fillOpacity);
        strokeOpacity ??= optionalNumber(rule.paint.strokeOpacity);
    }

    const visibleFill = fill !== undefined && !EMPTY_PAINT.has(fill) && (opacity ?? 1) * (fillOpacity ?? 1) > 0;
    const visibleStroke = stroke !== undefined && !EMPTY_PAINT.has(stroke) && (opacity ?? 1) * (strokeOpacity ?? 1) > 0;
    return { fill, stroke, opacity, fillOpacity, strokeOpacity, visible: visibleFill || visibleStroke, fillSource, strokeSource };
}

/** Measure exactly what paint the SVG carries. */
export function analyzeSvgPaint(svg: string): SvgPaintEvidence {
    const rules = classRules(svg);
    const fills = new Set<string>();
    const strokes = new Set<string>();
    const opened: Element[] = [];
    const chain: ResolvedPaint[] = [];
    let geometry = 0;
    let texts = 0;
    let painted = 0;
    let attributePainted = 0;
    let classPainted = 0;
    let erased = 0;

    for (const element of elements(svg)) {
        if (element.closing) {
            const index = opened.map((entry) => entry.name).lastIndexOf(element.name);
            if (index >= 0) {
                opened.splice(index);
                chain.splice(index);
            }
            continue;
        }
        const paint = resolve(element, opened, chain, rules);
        if (!element.selfClosing) {
            opened.push(element);
            chain.push(paint);
        }
        if (element.name === "text") {
            texts += 1;
            continue;
        }
        if (!GEOMETRY_TAGS.has(element.name)) {
            continue;
        }
        geometry += 1;
        if (paint.fill !== undefined && !EMPTY_PAINT.has(paint.fill)) {
            fills.add(paint.fill);
        }
        if (paint.stroke !== undefined && !EMPTY_PAINT.has(paint.stroke)) {
            strokes.add(paint.stroke);
        }
        if (paint.visible) {
            painted += 1;
            if (paint.fillSource === "attribute" || paint.strokeSource === "attribute") {
                attributePainted += 1;
            } else {
                classPainted += 1;
            }
        } else if (paint.fill !== undefined || paint.stroke !== undefined) {
            erased += 1;
        }
    }

    return {
        elements: geometry,
        texts,
        painted,
        attributePainted,
        classPainted,
        fills: [...fills].sort(),
        strokes: [...strokes].sort(),
        erased,
    };
}
