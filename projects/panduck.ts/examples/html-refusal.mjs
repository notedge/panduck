import { loadPanduckNode } from "@notedge/panduck"
import { readFixtureBytes } from "@notedge/panduck-fixtures"

const panduck = await loadPanduckNode()
const result = panduck.convertBytesMemory(
    "html",
    "markdown",
    "basic.html",
    Buffer.from(readFixtureBytes("notedown/basic.html")),
    "[]",
    "{}",
)
const report = JSON.parse(result.reportJson)

console.log(`exit ${result.exitCode} · status ${report.status}`)
for (const diagnostic of report.diagnostics ?? []) {
    console.log(`  ${diagnostic.code}: ${diagnostic.message}`)
}
if (result.exitCode === 0) {
    throw new Error("expected the html route to refuse this sample")
}
