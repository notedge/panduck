import { buildPanduckCli } from "./cli.js";

export async function runCli(argv: string[]): Promise<number> {
    try {
        return await buildPanduckCli().parse(argv);
    } catch (error) {
        console.error(`error: ${error instanceof Error ? error.message : String(error)}`);
        return 2;
    }
}
