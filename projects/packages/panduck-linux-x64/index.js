import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
export default require("./lib/panduck-linux-x64-gnu.node");
