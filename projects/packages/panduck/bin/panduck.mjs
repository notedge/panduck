#!/usr/bin/env node

import { createJiti } from "jiti";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const jiti = createJiti(import.meta.url, { interopDefault: true });
const { runCli } = await jiti.import(join(here, "../src/cli/main.ts"));

process.exit(await runCli(process.argv.slice(2)));
