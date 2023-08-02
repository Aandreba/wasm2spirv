import { init as wasiInit, WASI } from '@wasmer/wasi';

const ENCODER = new TextEncoder();
const DECODER = new TextDecoder("utf-8", {
    fatal: true,
    ignoreBOM: true
})

const NULLPTR = 0;
const PTR_SIZE = 4;
const PTR_LOG2_ALIGN = 2;

const W2S_STRING_SIZE = 2 * PTR_SIZE;
const W2S_STRING_LOG2_ALIGN = PTR_LOG2_ALIGN;


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
/** @type {WebAssembly.Memory} */
let memory;
/** @type {number} */
let string_bucket;
/** @type {FinalizationRegistry} */
let registry = new FinalizationRegistry(exec => (exec)());

/**
 * @param {BufferSource | Response | Promise<Response>} source
 */
export async function init(source) {
    await wasiInit();
    const wasi = new WASI({})

    /** @type {WebAssembly.Module} */
    let compiledModule;
    if (!source.buffer || source instanceof Promise) {
        compiledModule = await WebAssembly.compileStreaming(source);
    } else {
        compiledModule = await WebAssembly.compile(source);
    }

    instance = await wasi.instantiate(compiledModule)
    memory = instance.exports.memory;
    string_bucket = (instance.exports.w2s_malloc)(W2S_STRING_SIZE, W2S_STRING_LOG2_ALIGN);
}

/**
 * @extends {Error}
 */
export class W2SError {
    constructor() {
        (instance.exports.w2s_take_last_error_message)(string_bucket);
        this.message = importString(string_bucket);
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
        registry.register(this, () => (instance.exports.w2s_config_destroy)(ptr))
    }

    /**
     * Drops the `CompilationConfig` manually, instead of relying on the JavaScript garbage collector.
     */
    manuallyFree() {
        registry.unregister(this)
        (instance.exports.w2s_config_destroy)(this.ptr)
    }

    /**
     * Creates a new compilet config from a JSON file.
     * @param {string} json String containing the contents of the configuration, parsed in JSON.
     * @returns {CompilationConfig}
     */
    static fromJSON(json) {
        const [jsonBuffer, jsonLen] = exportString(json);

        let ptr;
        try {
            ptr = (instance.exports.w2s_config_from_json_string)(jsonBuffer.byteOffset, jsonLen);
            if (ptr === NULLPTR) throw new W2SError()
        } finally {
            (instance.exports.w2s_free)(jsonBuffer.byteOffset, jsonBuffer.byteLength, 0);
        }

        return new CompilationConfig(ptr)
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
     * @param {Uint8Array} bytes
     */
    constructor(config, bytes) {
        const [usedBytes, bytesCloned] = memory.buffer === bytes.buffer ? [bytes, false] : [copyBytes(bytes), true];
        const usedConfig = (instance.exports.w2s_config_clone)(config.ptr);
        console.log(usedBytes, bytesCloned, usedConfig)

        let ptr;
        try {
            ptr = (instance.exports.w2s_compilation_new)(usedConfig, usedBytes);
        } finally {
            if (bytesCloned) {
                (instance.exports.w2s_free)(usedBytes.byteOffset, usedBytes.byteLength, 0)
            }
        }

        if (ptr === NULLPTR) throw new W2SError()
        registry.register(this, () => (instance.exports.w2s_compilation_destroy)(ptr))
        this.ptr = ptr
    }

    /**
     * Drops the `CompilationConfig` manually, instead of relying on the JavaScript garbage collector.
     */
    manuallyFree() {
        registry.unregister(this)
        (instance.exports.w2s_compilation_destroy)(this.ptr)
    }
}

/**
 * Exports a JavaScript string into a UTF-8 encoded WebAssembly string.
 * @param {string} str
 * @returns {[Uint8Array, number]}
 */
function exportString(str) {
    const bufferLen = 3 * str.length;
    /** @type {number} */
    const ptr = (instance.exports.w2s_malloc)(bufferLen, 0);

    try {
        const buffer = new Uint8Array(memory.buffer, ptr, bufferLen)
        const len = ENCODER.encodeInto(str, buffer);
        return [buffer, len.written ?? 0]
    } catch (e) {
        (instance.exports.w2s_free)(ptr, bufferLen, 0);
        throw e;
    }
}

/**
 * Imports a WebAssembly UTF-8 string to a JavaScript string.
 * @param {number} ptr Pointer to `w2s_string` object.
 * This underlying string will be deallocated by this function, even if it throws.
 * @returns {string | null}
 */
function importString(ptr) {
    const resultView = new DataView(memory.buffer, ptr, W2S_STRING_SIZE)
    const strPtr = resultView.getInt32(0, true)
    const strLen = resultView.getInt32(PTR_SIZE, true)

    let result;
    try {
        if (strPtr === NULLPTR) result = null
        else result = DECODER.decode(new Uint8Array(memory.buffer, strPtr, strLen))
    } finally {
        (instance.exports.w2s_string_destroy)(ptr)
    }

    return result
}

/**
 * Clones bytes into WebAssembly memory.
 * @param {Uint8Array} bytes
 * @returns {Uint8Array}
 */
function copyBytes(bytes) {
    const newPtr = (instance.exports.w2s_malloc)(bytes.byteLength, 0);
    const newBytes = new Uint8Array(memory.buffer, newPtr, bytes.byteLength);

    try {
        newBytes.set(bytes, 0);
    } catch (e) {
        (instance.exports.w2s_free)(newPtr, bytes.byteLength, 0);
        throw e;
    }

    return newBytes
}
