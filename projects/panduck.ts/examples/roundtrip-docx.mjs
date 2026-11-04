import { loadPanduckNode } from "@notedge/panduck"
import { readFixtureText } from "@notedge/panduck-fixtures"

const panduck = await loadPanduckNode()
const source = readFixtureText("convert/roundtrip.md")
const forward = panduck.convertBytesMemory(
    "markdown",
    "docx",
    "roundtrip.md",
    Buffer.from(source, "utf8"),
    "[]",
    "{}",
)
if (forward.exitCode !== 0 || !forward.binary) {
    throw new Error("markdown to docx produced no binary output")
}
const back = panduck.convertBytesMemory(
    "docx",
    "markdown",
    "roundtrip.back.docx",
    Buffer.from(forward.binary),
    "[]",
    "{}",
)
const report = JSON.parse(back.reportJson)

console.log(`exit ${back.exitCode} · status ${report.status}`)
console.log(back.text.slice(0, 400))
