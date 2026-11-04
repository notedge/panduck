/** PDF glyph geometry derived from `pdfjs-dist` and compared with layout reserved widths. */

import type { PdfLayoutRun, PdfRunAdvance } from "./types.js";

/** pdf.js legacy ESM build; the regular build warns when it runs under Node. */
const PDFJS_LEGACY_BUILD = "pdfjs-dist/legacy/build/pdf.mjs";

type PdfGlyph = {
    readonly unicode?: string;
    /** Glyph advance in 1/1000 text space units. */
    readonly width?: number;
    readonly fontChar?: number | null;
};

type PdfOperatorList = {
    readonly fnArray: readonly number[];
    readonly argsArray: readonly unknown[][];
};

type PdfPage = {
    getOperatorList(): Promise<PdfOperatorList>;
};

type PdfDocument = {
    readonly numPages: number;
    getPage(index: number): Promise<PdfPage>;
    destroy(): Promise<void>;
};

type PdfJsModule = {
    readonly OPS: Readonly<Record<string, number>>;
    getDocument(params: {
        readonly data: Uint8Array;
        readonly verbosity?: number;
        readonly useSystemFonts?: boolean;
    }): { readonly promise: Promise<PdfDocument> };
};

/** One painted glyph with its origin and advance in PDF points. */
type PaintedGlyph = {
    readonly text: string;
    readonly x: number;
    readonly y: number;
    readonly advance: number;
    readonly page: number;
};

type TextState = {
    /** Text matrix `[a, b, c, d, e, f]`. */
    matrix: [number, number, number, number, number, number];
    /** Text line matrix, the anchor `moveText` and `nextLine` adjust. */
    lineMatrix: [number, number, number, number, number, number];
    fontSize: number;
    leading: number;
    charSpacing: number;
    wordSpacing: number;
    horizontalScale: number;
};

type GraphicsState = Pick<TextState, "fontSize" | "leading" | "charSpacing" | "wordSpacing" | "horizontalScale">;

function freshTextState(): TextState {
    return {
        matrix: [1, 0, 0, 1, 0, 0],
        lineMatrix: [1, 0, 0, 1, 0, 0],
        fontSize: 0,
        leading: 0,
        charSpacing: 0,
        wordSpacing: 0,
        horizontalScale: 1,
    };
}

function translate(matrix: TextState["matrix"], tx: number, ty: number): TextState["matrix"] {
    const [a, b, c, d, e, f] = matrix;
    return [a, b, c, d, e + tx * a + ty * c, f + tx * b + ty * d];
}

function isSpaceGlyph(glyph: PdfGlyph): boolean {
    return glyph.unicode === " " || glyph.fontChar === 32;
}

function glyphAdvance(state: TextState, glyph: PdfGlyph): number {
    const width = typeof glyph.width === "number" && Number.isFinite(glyph.width) ? glyph.width : 0;
    const wordSpacing = isSpaceGlyph(glyph) ? state.wordSpacing : 0;
    return ((width / 1000) * state.fontSize + state.charSpacing + wordSpacing) * state.horizontalScale;
}

/**
 * Collects painted glyph origins from a PDF.
 *
 * Origins come from the PDF text matrix and per glyph advances, never from
 * pdf.js text items: MuPDF style writers may insert synthetic spaces, and
 * `getTextContent` can merge or split items by font, so only the glyph sequence
 * keeps a stable character order.
 */
async function paintedGlyphs(pdf: Uint8Array, pdfjs: PdfJsModule): Promise<readonly PaintedGlyph[]> {
    const glyphs: PaintedGlyph[] = [];
    const document = await pdfjs.getDocument({ data: pdf, verbosity: 0 }).promise;
    try {
        const opNames = new Map<number, string>();
        for (const [name, code] of Object.entries(pdfjs.OPS)) {
            opNames.set(code, name);
        }
        for (let pageIndex = 1; pageIndex <= document.numPages; pageIndex += 1) {
            const page = await document.getPage(pageIndex);
            const operators = await page.getOperatorList();
            walkOperators(operators, opNames, glyphs, pageIndex);
        }
    } finally {
        await document.destroy();
    }
    return glyphs;
}

function walkOperators(
    operators: PdfOperatorList,
    opNames: Map<number, string>,
    glyphs: PaintedGlyph[],
    page: number,
): void {
    const state = freshTextState();
    const stack: GraphicsState[] = [];
    const record = (glyph: PdfGlyph): void => {
        const text = typeof glyph.unicode === "string" ? glyph.unicode : "";
        if (text.length === 0) {
            return;
        }
        const advance = glyphAdvance(state, glyph);
        glyphs.push({ text, x: state.matrix[4], y: state.matrix[5], advance, page });
        state.matrix = translate(state.matrix, advance, 0);
    };
    for (let index = 0; index < operators.fnArray.length; index += 1) {
        const name = opNames.get(operators.fnArray[index]);
        const args = operators.argsArray[index] ?? [];
        switch (name) {
            case "beginText":
                state.matrix = [1, 0, 0, 1, 0, 0];
                state.lineMatrix = [1, 0, 0, 1, 0, 0];
                break;
            case "setTextMatrix": {
                const [a, b, c, d, e, f] = args as [number, number, number, number, number, number];
                state.matrix = [a, b, c, d, e, f];
                state.lineMatrix = [a, b, c, d, e, f];
                break;
            }
            case "moveText": {
                const [tx, ty] = args as [number, number];
                state.lineMatrix = translate(state.lineMatrix, tx, ty);
                state.matrix = [...state.lineMatrix];
                break;
            }
            case "setLeadingMoveText": {
                const [tx, ty] = args as [number, number];
                state.leading = -ty;
                state.lineMatrix = translate(state.lineMatrix, tx, ty);
                state.matrix = [...state.lineMatrix];
                break;
            }
            case "setLeading":
                state.leading = args[0] as number;
                break;
            case "nextLine":
                state.lineMatrix = translate(state.lineMatrix, 0, -state.leading);
                state.matrix = [...state.lineMatrix];
                break;
            case "setFont":
                state.fontSize = args[1] as number;
                break;
            case "setCharSpacing":
                state.charSpacing = args[0] as number;
                break;
            case "setWordSpacing":
                state.wordSpacing = args[0] as number;
                break;
            case "setHScale":
                state.horizontalScale = (args[0] as number) / 100;
                break;
            case "showText":
                for (const glyph of args[0] as readonly PdfGlyph[]) {
                    record(glyph);
                }
                break;
            case "showSpacedText":
                for (const entry of args[0] as readonly (PdfGlyph | number)[]) {
                    if (typeof entry === "number") {
                        state.matrix = translate(state.matrix, (-entry / 1000) * state.fontSize * state.horizontalScale, 0);
                    } else {
                        record(entry);
                    }
                }
                break;
            case "nextLineShowText":
                state.lineMatrix = translate(state.lineMatrix, 0, -state.leading);
                state.matrix = [...state.lineMatrix];
                for (const glyph of args[0] as readonly PdfGlyph[]) {
                    record(glyph);
                }
                break;
            case "nextLineSetSpacingShowText":
                state.lineMatrix = translate(state.lineMatrix, 0, -state.leading);
                state.matrix = [...state.lineMatrix];
                state.wordSpacing = args[0] as number;
                state.charSpacing = args[1] as number;
                for (const glyph of args[2] as readonly PdfGlyph[]) {
                    record(glyph);
                }
                break;
            case "save":
                stack.push({
                    fontSize: state.fontSize,
                    leading: state.leading,
                    charSpacing: state.charSpacing,
                    wordSpacing: state.wordSpacing,
                    horizontalScale: state.horizontalScale,
                });
                break;
            case "restore": {
                const saved = stack.pop();
                if (saved) {
                    Object.assign(state, saved);
                }
                break;
            }
            default:
                break;
        }
    }
}

type Match = { readonly start: number; readonly end: number };

/** Finds `text` as a subsequence of painted glyphs starting at or after `from`. */
function matchRun(glyphs: readonly PaintedGlyph[], text: string, from: number): Match | null {
    for (let start = from; start < glyphs.length; start += 1) {
        let glyphIndex = start;
        let textIndex = 0;
        while (textIndex < text.length && glyphIndex < glyphs.length) {
            const glyphText = glyphs[glyphIndex].text;
            if (glyphText.length !== 1) {
                break;
            }
            if (glyphText !== text[textIndex]) {
                break;
            }
            glyphIndex += 1;
            textIndex += 1;
        }
        if (textIndex === text.length) {
            return { start, end: glyphIndex };
        }
    }
    return null;
}

/**
 * Measures each layout run against the PDF.
 *
 * A run's advance is the distance from its first painted glyph origin to the
 * next painted glyph origin, which is the follower run's first glyph. Both
 * endpoints are explicit PDF text matrix positions, so the comparison does not
 * accumulate per glyph width quantisation.
 */
export function measureRunAdvances(
    glyphs: readonly PaintedGlyph[],
    runs: readonly PdfLayoutRun[],
): readonly PdfRunAdvance[] {
    const results: PdfRunAdvance[] = [];
    let cursor = 0;
    for (const run of runs) {
        const match = matchRun(glyphs, run.text, cursor);
        if (!match) {
            results.push({
                runText: run.text,
                style: run.style,
                fontFamily: run.fontFamily,
                page: run.page,
                startX: null,
                nextX: null,
                reservedWidth: run.width,
                paintedAdvance: null,
                delta: null,
                note: "run text not found in the painted glyph sequence",
            });
            continue;
        }
        const follower = glyphs[match.end];
        const startX = glyphs[match.start].x;
        const nextX = follower?.x ?? null;
        const paintedAdvance = nextX === null ? null : nextX - startX;
        results.push({
            runText: run.text,
            style: run.style,
            fontFamily: run.fontFamily,
            page: run.page,
            startX,
            nextX,
            reservedWidth: run.width,
            paintedAdvance,
            delta: paintedAdvance === null ? null : Math.abs(paintedAdvance - run.width),
            note: follower === undefined ? "run ends the painted glyph sequence" : undefined,
        });
        cursor = match.end;
    }
    return results;
}

/** Loads the PDF through `pdfjs-dist` and measures every layout run in one pass. */
export async function probePdfGeometry(
    pdf: Uint8Array,
    runs: readonly PdfLayoutRun[],
): Promise<{ readonly runs: readonly PdfRunAdvance[]; readonly glyphs: number; readonly pages: number }> {
    const pdfjs = (await import(PDFJS_LEGACY_BUILD)) as PdfJsModule;
    const glyphs = await paintedGlyphs(pdf, pdfjs);
    const pages = glyphs.reduce((maximum, glyph) => Math.max(maximum, glyph.page), 0);
    return { runs: measureRunAdvances(glyphs, runs), glyphs: glyphs.length, pages };
}
