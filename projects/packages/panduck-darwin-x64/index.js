import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
export default require("./lib/panduck-darwin-x64.node");
