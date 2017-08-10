import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { createCli } from "@vmz/commander";

import { registerCheckCommand } from "./commands/check-cmd.js";
import { registerConvertCommand } from "./commands/convert-cmd.js";
import { registerDoctorCommand } from "./commands/doctor-cmd.js";
import { registerFormatsCommand } from "./commands/formats-cmd.js";
import { registerInspectCommand } from "./commands/inspect-cmd.js";
import { registerPlanCommand } from "./commands/plan-cmd.js";

export function buildPanduckCli() {
    const localesRoot = join(dirname(fileURLToPath(import.meta.url)), "../../locales");
    const cli = createCli("panduck")
        .locales(localesRoot, { envKeys: ["PANDUCK_LOCALE", "LOCALE", "LANG", "LC_ALL"] })
        .intro("cli.intro");

    registerConvertCommand(cli);
    registerPlanCommand(cli);
    registerInspectCommand(cli);
    registerCheckCommand(cli);
    registerFormatsCommand(cli);
    registerDoctorCommand(cli);

    return cli;
}
