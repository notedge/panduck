import { loadPanduckNode } from "@notedge/panduck"
import { readFixtureText } from "@notedge/panduck-fixtures"

const panduck = await loadPanduckNode()
const source = readFixtureText("convert/roundtrip.md")
const forward = panduck.convertBytesMemory(
    "markdown",
    "pdf",
    "roundtrip.md",
    Buffer.from(source, "utf8"),
    "[]",
    "{}",
)
if (forward.exitCode !== 0 || !forward.binary) {
    throw new Error("markdown to pdf produced no binary output")
}
const back = panduck.convertBytesMemory(
    "pdf",
    "markdown",
    "roundtrip.back.pdf",
    Buffer.from(forward.binary),
    "[]",
    "{}",
)
const report = JSON.parse(back.reportJson)

console.log(`exit ${back.exitCode} · status ${report.status} · text ${back.text.length}`)
console.log(back.text.slice(0, 400))
