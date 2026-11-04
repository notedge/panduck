import { readFileSync } from "node:fs"
import { fileURLToPath } from "node:url"

import { loadPanduckNode } from "@notedge/panduck"

const inputPath = fileURLToPath(new URL("./input.md", import.meta.url))
const panduck = await loadPanduckNode()
const source = readFileSync(inputPath)
const result = panduck.convertBytesMemory("markdown", "html", "input.md", source, "[]", "{}")
const report = JSON.parse(result.reportJson)

console.log(`exit ${result.exitCode} · status ${report.status}`)
console.log(result.html)
