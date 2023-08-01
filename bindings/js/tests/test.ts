import { init, CompilationConfig } from "../index.js"

const wasm2spirv = await Deno.readFile("../../target/wasm32-wasi/release/wasm2spirv_c.wasm");
await init(wasm2spirv);

Deno.test("saxpy", async () => {
    const [saxpy_config, saxpy_bytes] = await Promise.all([
        Deno.readTextFile("../../examples/saxpy/saxpy.json"),
        Deno.readFile("../../examples/saxpy/saxpy.wasm")
    ])

    const config = CompilationConfig.fromJSON(saxpy_config);
})
