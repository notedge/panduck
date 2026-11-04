import { spawnSync } from "node:child_process"
import { fileURLToPath } from "node:url"

const cli = fileURLToPath(new URL("../../packages/panduck-harness/bin/cli.mjs", import.meta.url))
const result = spawnSync(process.execPath, [cli, "verify"], { stdio: "inherit" })

process.exitCode = result.status ?? 1
