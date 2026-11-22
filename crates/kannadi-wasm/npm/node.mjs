// Node entry: loads the WebAssembly module synchronously, so the exports work on import.
import { readFileSync } from "node:fs";
import { initSync } from "./kannadi_wasm.js";

initSync({ module: readFileSync(new URL("./kannadi_wasm_bg.wasm", import.meta.url)) });

export * from "./kannadi_wasm.js";
