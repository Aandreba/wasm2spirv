import {describe, expect, test} from '@jest/globals';
import { init, CompilationConfig, Compilation } from "../index.js"
import { readFile } from "node:fs/promises"
import "expose-gc"

async function initialize() {
    const wasm2spirv = await readFile("../../target/wasm32-wasi/release/wasm2spirv_c.wasm");
    await init(wasm2spirv);
}

test("saxpy", async () => {
    await initialize();

    const [saxpy_config, saxpy_bytes] = await Promise.all([
        readFile("../../examples/saxpy/saxpy.json", {
            encoding: "utf-8"
        }),
        readFile("../../examples/saxpy/saxpy.wasm")
    ])

    let config: CompilationConfig = CompilationConfig.fromJSON(saxpy_config);
    let compiled: Compilation = new Compilation(config, new Uint8Array(saxpy_bytes.buffer));
    console.log(compiled);
})

/**
 * @param {number} millis Milisseconds to sleep
 * @returns {Promise<void>}
 */
function sleep(millis) {
    return new Promise(resolve => {
        setTimeout(resolve, millis)
    })
}
