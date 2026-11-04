/**
 * Produces the docx / pdf / markdown artifacts that `verify.py` reads.
 *
 * The split is deliberate: Node owns the Panduck route, Python owns the
 * libraries that judge the product. A Panduck bug therefore cannot pass the
 * Python checks by agreeing with itself.
 */
import { spawnSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { loadPanduckNode } from "@notedge/panduck";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(tmpdir(), "panduck-crosscheck-python");
mkdirSync(root, { recursive: true });

const fixtures = join(here, "..", "..", "packages", "panduck-fixtures", "fixtures");
const panduck = loadPanduckNode();

function convert(from, to, name, fixturePath) {
    const bytes = new Uint8Array(readFileSync(join(fixtures, fixturePath)));
    const result = panduck.convertBytesMemory(from, to, name, bytes, "[]", "{}");
    if (result.exitCode !== 0) {
        console.error(`${name}: exit ${result.exitCode}\n${result.reportJson}`);
        process.exit(1);
    }
    return result;
}

writeFileSync(join(root, "table-and-list.docx"), convert("html", "docx", "table-and-list.html", "convert/table-and-list.html").binary);
writeFileSync(join(root, "nested-structures.docx"), convert("markdown", "docx", "nested-structures.md", "convert/nested-structures.md").binary);
writeFileSync(join(root, "fonts-and-cjk.pdf"), convert("markdown", "pdf", "fonts-and-cjk.md", "convert/fonts-and-cjk.md").binary);
writeFileSync(join(root, "table-and-list.md"), convert("html", "markdown", "table-and-list.html", "convert/table-and-list.html").text);

const python = join(here, ".venv", "Scripts", "python.exe");
const run = spawnSync(python, [join(here, "verify.py"), root], { stdio: "inherit" });
if (run.error) {
    console.error(`python oracle unavailable at ${python}: ${run.error.message}`);
    console.error("create it with `python -m venv .venv` then `pip install -r requirements.txt`");
    process.exit(1);
}
process.exit(run.status ?? 1);
