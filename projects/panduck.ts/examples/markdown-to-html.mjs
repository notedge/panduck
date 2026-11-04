import { loadPanduckNode } from "@notedge/panduck"
import { readFixtureText } from "@notedge/panduck-fixtures"

const panduck = await loadPanduckNode()
const source = readFixtureText("convert/diagrams.md")
const result = panduck.convertBytesMemory(
    "markdown",
    "html",
    "diagrams.md",
    Buffer.from(source, "utf8"),
    "[]",
    "{}",
)
const report = JSON.parse(result.reportJson)
const svgCount = (result.html.match(/<svg/g) ?? []).length

console.log(`exit ${result.exitCode} · status ${report.status} · svg ${svgCount}`)
if (result.exitCode !== 0 || svgCount === 0) {
    throw new Error("markdown to html did not render the diagram fences")
}
