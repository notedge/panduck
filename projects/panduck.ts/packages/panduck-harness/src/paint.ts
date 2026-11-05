/**
 * Paint analysis for one rendered diagram SVG.
 *
 * A non-empty `<svg>` says nothing about visibility: the DaVinci theme paints
 * shape roles through CSS class rules, an authored scene paints through
 * attributes, and a scene with no role palette at all paints through neither.
 * The harness resolves both sources and counts geometry that actually carries a
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
    /** Painted through a matching class rule in a `<style>` block. */
    readonly classPainted: number;
    /** Distinct visible fill values. */
    readonly fills: readonly string[];
    /** Distinct visible stroke values. */
    readonly strokes: readonly string[];
    /** Shape elements whose resolved paint is fully transparent. */
    readonly erased: number;
};

const TAG = /<([a-zA-Z][a-zA-Z0-9-]*)((?:[^>"']|"[^"]*"|'[^']*')*)>/g;
const ROOT_TAG = /<([a-zA-Z][a-zA-Z0-9-]*)((?:[^>"']|"[^"]*"|'[^']*')*)>/;
const ATTRIBUTE = /([a-zA-Z_:][a-zA-Z0-9_:.-]*)="([^"]*)"/g;
const STYLE_BLOCK = /<style[^>]*>([\s\S]*?)<\/style>/g;

/** One resolved paint declaration, with the source of each property. */
type ResolvedPaint = {
    readonly fill?: string;
    readonly stroke?: string;
    readonly fillFromClass: boolean;
    readonly strokeFromClass: boolean;
    readonly visible: boolean;
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

/** Collect the class tokens of one compound selector such as `.a.b`. */
function compoundClasses(compound: string): readonly string[] {
    return [...compound.matchAll(/\.([a-zA-Z0-9_-]+)/g)].map((match) => match[1]);
}

/** A `<style>` rule reduced to the classes an element must carry. */
type ClassRule = {
    readonly own: readonly string[];
    readonly ancestors: readonly string[];
    readonly paint: Partial<Record<"fill" | "stroke" | "opacity" | "fillOpacity" | "strokeOpacity", string>>;
};

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
            if (!selector.startsWith(".")) {
                continue;
            }
            const compounds = selector.split(/\s+/).filter((part) => part.startsWith("."));
            if (compounds.length === 0) {
                continue;
            }
            const entries = declarations(rule.slice(open + 1));
            const paint: ClassRule["paint"] = {
                fill: entries.get("fill"),
                stroke: entries.get("stroke"),
                opacity: entries.get("opacity"),
                fillOpacity: entries.get("fill-opacity"),
                strokeOpacity: entries.get("stroke-opacity"),
            };
            rules.push({
                own: compoundClasses(compounds[compounds.length - 1]),
                ancestors: compounds.slice(0, -1).flatMap(compoundClasses),
                paint,
            });
        }
    }
    return rules.filter((rule) => rule.own.length > 0);
}

function optionalNumber(value: string | undefined): number | undefined {
    if (value === undefined) {
        return undefined;
    }
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : undefined;
}

/** Resolve the paint of one element: attributes win, class rules fill the gaps. */
function resolve(
    elementClasses: readonly string[],
    rootClasses: readonly string[],
    attributes: Map<string, string>,
    rules: readonly ClassRule[],
): ResolvedPaint {
    let fill = attributes.get("fill");
    let stroke = attributes.get("stroke");
    let opacity = optionalNumber(attributes.get("opacity"));
    let fillOpacity = optionalNumber(attributes.get("fill-opacity"));
    let strokeOpacity = optionalNumber(attributes.get("stroke-opacity"));
    let fillFromClass = false;
    let strokeFromClass = false;

    for (const rule of rules) {
        const own = rule.own.every((name) => elementClasses.includes(name));
        const scoped = rule.ancestors.every((name) => elementClasses.includes(name) || rootClasses.includes(name));
        if (!own || !scoped) {
            continue;
        }
        if (fill === undefined && rule.paint.fill !== undefined) {
            fill = rule.paint.fill;
            fillFromClass = true;
        }
        if (stroke === undefined && rule.paint.stroke !== undefined) {
            stroke = rule.paint.stroke;
            strokeFromClass = true;
        }
        opacity ??= optionalNumber(rule.paint.opacity);
        fillOpacity ??= optionalNumber(rule.paint.fillOpacity);
        strokeOpacity ??= optionalNumber(rule.paint.strokeOpacity);
    }

    const visibleFill = fill !== undefined && !EMPTY_PAINT.has(fill) && (opacity ?? 1) * (fillOpacity ?? 1) > 0;
    const visibleStroke = stroke !== undefined && !EMPTY_PAINT.has(stroke) && (opacity ?? 1) * (strokeOpacity ?? 1) > 0;
    return { fill, stroke, fillFromClass, strokeFromClass, visible: visibleFill || visibleStroke };
}

/** Attributes of the root element, used to resolve a leading compound selector. */
function rootAttributes(svg: string): Map<string, string> {
    const attributes = new Map<string, string>();
    const root = ROOT_TAG.exec(svg);
    if (root === null) {
        return attributes;
    }
    for (const attribute of root[2].matchAll(ATTRIBUTE)) {
        attributes.set(attribute[1], attribute[2]);
    }
    return attributes;
}

/** Measure exactly what paint the SVG carries. */
export function analyzeSvgPaint(svg: string): SvgPaintEvidence {
    const rules = classRules(svg);
    const rootClasses = rootAttributes(svg).get("class")?.split(/\s+/).filter(Boolean) ?? [];
    const fills = new Set<string>();
    const strokes = new Set<string>();
    let elements = 0;
    let texts = 0;
    let painted = 0;
    let attributePainted = 0;
    let classPainted = 0;
    let erased = 0;

    for (const tag of svg.matchAll(TAG)) {
        const name = tag[1].toLowerCase();
        if (!GEOMETRY_TAGS.has(name) && name !== "text") {
            continue;
        }
        const attributes = new Map<string, string>();
        for (const attribute of tag[2].matchAll(ATTRIBUTE)) {
            attributes.set(attribute[1], attribute[2]);
        }
        if (name === "text") {
            texts += 1;
            continue;
        }
        elements += 1;
        const classes = attributes.get("class")?.split(/\s+/).filter(Boolean) ?? [];
        const paint = resolve(classes, rootClasses, attributes, rules);
        if (paint.fill !== undefined && !EMPTY_PAINT.has(paint.fill) && (paint.visible || paint.stroke === undefined)) {
            fills.add(paint.fill);
        }
        if (paint.stroke !== undefined && !EMPTY_PAINT.has(paint.stroke)) {
            strokes.add(paint.stroke);
        }
        if (paint.visible) {
            painted += 1;
            const attributeSource =
                (paint.fill !== undefined && !paint.fillFromClass) || (paint.stroke !== undefined && !paint.strokeFromClass);
            if (attributeSource) {
                attributePainted += 1;
            } else {
                classPainted += 1;
            }
        } else if (paint.fill !== undefined || paint.stroke !== undefined) {
            erased += 1;
        }
    }

    return {
        elements,
        texts,
        painted,
        attributePainted,
        classPainted,
        fills: [...fills].sort(),
        strokes: [...strokes].sort(),
        erased,
    };
}
