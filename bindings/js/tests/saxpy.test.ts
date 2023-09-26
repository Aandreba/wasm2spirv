import { test } from "@jest/globals"
import {
    init,
    CompilationConfig,
    Compilation,
    CompilationConfigBuilder,
    Target,
    Capabilities,
    FunctionConfigBuilder,
} from "../index.js"
import { readFile } from "node:fs/promises"

async function initialize() {
    const wasm2spirv = await readFile(
        "../../target/wasm32-wasi/release/wasm2spirv_c.wasm"
    )
    await init(wasm2spirv)
}

test("saxpy", async () => {
    await initialize()

    const [saxpy_config, saxpy_buffer] = await Promise.all([
        readFile("../../examples/saxpy/saxpy.json", {
            encoding: "utf-8",
        }),
        readFile("../../examples/saxpy/saxpy.wasm"),
    ])
    const saxpy_bytes = new Uint8Array(saxpy_buffer.buffer)

    const manualConfig = manualSaxpyConfig()
    const manualCompilation = new Compilation(manualConfig, saxpy_bytes)

    const config = CompilationConfig.fromJSON(saxpy_config)
    const compiled = new Compilation(config, saxpy_bytes)

    expect(manualCompilation.byteView).toEqual(compiled.byteView)

    console.log(compiled.assembly())
    console.log(compiled.msl())
})

function manualSaxpyConfig(): CompilationConfig {
    const builder = new CompilationConfigBuilder(
        Target.create("vulkan", { major: 1, minor: 1 }),
        Capabilities.create("dynamic", ["VariablePointers"]),
        ["VH_KHR_variable_pointers"],
        "logical",
        "GLSL450"
    )

    const saxpy = new FunctionConfigBuilder()
        .addExecutionMode("LocalSize", 1, 1, 1)
        .buildAndDestroy()
    console.log(saxpy)

    return builder.buildAndDestroy()
}
