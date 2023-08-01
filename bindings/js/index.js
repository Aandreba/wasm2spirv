import { init as wasiInit, WASI } from '@wasmer/wasi';

const ENCODER = new TextEncoder();

/*
(export "w2s_take_last_error_message" (func 6940))
(export "w2s_string_clone" (func 6941))
(export "w2s_string_destroy" (func 6942))
(export "w2s_config_from_json_string" (func 6943))
(export "w2s_config_from_json_fd" (func 6944))
(export "w2s_config_builder_new" (func 6945))
(export "w2s_config_builder_build" (func 6946))
(export "w2s_config_builder_destroy" (func 6947))
(export "w2s_config_destroy" (func 6948))
(export "w2s_compilation_new" (func 6949))
(export "w2s_compilation_assembly" (func 6950))
(export "w2s_compilation_words" (func 6951))
(export "w2s_compilation_bytes" (func 6952))
(export "w2s_compilation_glsl" (func 6953))
(export "w2s_compilation_hlsl" (func 6954))
(export "w2s_compilation_msl" (func 6955))
(export "w2s_compilation_wgsl" (func 6956))
(export "w2s_compilation_destroy" (func 6957))
*/

/** @type {WebAssembly.Instance} */
let instance;
/** @type {WebAssembly.Module} */
let module;
/** @type {FinalizationRegistry} */
let registry;

/**
 * @param {BufferSource | Response | Promise<Response>} source
 */
export async function init(source) {
    await wasiInit();
    const wasi = new WASI({})

    /** @type {WebAssembly.Module} */
    let compiledModule;
    if ("arrayBuffer" in source || source instanceof Promise) {
        compiledModule = await WebAssembly.compileStreaming(source);
    } else {
        compiledModule = await WebAssembly.compile(source);
    }

    instance = await wasi.instantiate(compiledModule)
    module = compiledModule
    registry = new FinalizationRegistry(([ptr, destructor]) => (destructor)(ptr))
}

/**
 * @extends {Error}
 */
export class W2SError {
    constructor() {
        const str = 0;
    }
}

export class CompilationConfig {
    /**
     * @readonly
     * @type {number}
     * */
    ptr;

    /**
     * @private
     * @param {number} ptr
     */
    constructor(ptr) {
        this.ptr = ptr;
        registry.register(this, [ptr, instance.exports.w2s_config_destroy])
    }

    /**
     * Creates a new compilet config from a JSON file
     * @param {string} json
     * @returns {CompilationConfig}
     */
    static fromJSON(json) {
        const [jsonBuffer, jsonLen] = exportString(json);
        try {
            const ptr = (instance.exports.w2s_config_from_json_string)(jsonBuffer.byteOffset, jsonLen);
            if (ptr === 0) throw new
        } finally {
            (instance.exports.w2s_free)(jsonBuffer.byteOffset, jsonBuffer.byteLength, 1);
        }
    }
}

export class Compilation {
    /**
     * @readonly
     * @type {number}
     * */
    ptr;

    /**
     * @param {CompilationConfig} config
     */
    constructor(config) {
        registry.unregister(config);
        this.ptr = (instance.exports.w2s_compilation_new)(config.ptr);
        registry.register(this, [ptr, instance.exports.w2s_compilation_destroy])
    }
}

/**
 * Exports a JavaScript string into a UTF-8 encoded WebAssembly string
 * @param {string} str
 * @returns {[Uint8Array, number]}
 */
function exportString(str) {
    const bufferLen = 3 * str.length;
    /** @type {number} */
    const ptr = (instance.exports.w2s_malloc)(bufferLen, 1);

    try {
        const buffer = new Uint8Array(instance.exports.memory.buffer, ptr, bufferLen)
        const len = ENCODER.encodeInto(str, buffer);
        return [buffer, len.written ?? 0]
    } catch (e) {
        (instance.exports.w2s_free)(ptr, bufferLen, 1);
        throw e;
    }
}
