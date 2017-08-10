import type { Cli, ParsedOptions } from "@vmz/commander";

import { createContext } from "../context.js";
import { emitJson } from "../diagnostics.js";
import { findFormat, type OperationKind } from "../format-registry.js";
import { readSharedOptions, readString } from "../options.js";

export function registerFormatsCommand(cli: Cli): void {
    cli.command("formats", "cli.cmd.formats")
        .option("--operation <kind>", "cli.opt.operation")
        .option("--json", "cli.opt.json")
        .action((options: ParsedOptions) => runFormats(options));
}

function runFormats(options: ParsedOptions): number {
    const shared = readSharedOptions(options);
    const ctx = createContext();
    const formatId = options._[0] ?? readString(options, "format");
    const operation = readString(options, "operation") as OperationKind | undefined;
    const selected = formatId ? findFormat(ctx.registry, formatId) : undefined;
    if (formatId && !selected) {
        console.error(`unknown format: ${formatId}`);
        return 2;
    }

    const records = selected ? [selected] : ctx.registry;
    const payload = records.map((record) => {
        if (operation === "read") {
            return { format: record.format, reader: record.reader, container: record.container, coverage: record.coverage };
        }
        if (operation === "write") {
            return { format: record.format, writer: record.writer, container: record.container, coverage: record.coverage };
        }
        return record;
    });

    if (shared.json || options.json === true) {
        emitJson({ schema_version: "panduck.formats/v1", formats: payload });
        return 0;
    }

    for (const record of records) {
        console.log(`${record.format}\tread=${record.reader.state}\twrite=${record.writer.state}`);
    }
    return 0;
}
