/**
 * @typedef {("logical" | "physical" | "physical_storage_buffer")} W2SAddressingModel
 * @typedef {("simple" | "glsl450" | "opencl" | "vulkan")} W2SMemoryModel
 * @typedef {("universal" | "vulkan")} W2STargetPlatform
 * @typedef {("static" | "dynamic")} W2SCapabilityModel
 *
 * @typedef {object} W2SVersion
 * @property {number} major
 * @property {number} minor
 *
 * @typedef {object} W2STarget
 * @property {W2STargetPlatform} platform
 * @property {W2SVersion} version
 *
 * @typedef {object} W2SCapabilities
 * @property {W2SCapabilityModel} model
 * @property {number[]} capabilities
 */

import { init as wasiInit, WASI } from '@wasmer/wasi';

const ENCODER = new TextEncoder();
const DECODER = new TextDecoder("utf-8", {
    fatal: true,
    ignoreBOM: true
})

const NULLPTR = 0;

const PTR_SIZE = 4;
const PTR_LOG2_ALIGN = 2;

const W2S_VIEW_SIZE = 2 * PTR_SIZE;
const W2S_VIEW_LOG2_ALIGN = PTR_LOG2_ALIGN;

const W2S_VERSION_SIZE = 2;
const W2S_VERSION_LOG2_ALIGN = 0;

const W2S_TARGET_SIZE = 0;
const W2S_TARGET_LOG2_ALIGN = 0;

/** @type {WASI} */
let wasi;
/** @type {WebAssembly.Instance} */
let instance;
/** @type {WebAssembly.Memory} */
let memory;
/** @type {number} */
let view_bucket;
/** @type {FinalizationRegistry} */
let registry = new FinalizationRegistry(exec => (exec)());

/**
 * @param {BufferSource | Response | Promise<Response>} source
 */
export async function init(source) {
    if (wasi !== undefined) {
        console.warn("wasm2spirv has already been initialized")
        return
    }

    await wasiInit();
    wasi = new WASI({})

    /** @type {WebAssembly.Module} */
    let compiledModule;
    if (!source.buffer || source instanceof Promise) {
        compiledModule = await WebAssembly.compileStreaming(source);
    } else {
        compiledModule = await WebAssembly.compile(source);
    }

    instance = await wasi.instantiate(compiledModule)
    memory = instance.exports.memory;
    view_bucket = (instance.exports.w2s_malloc)(W2S_VIEW_SIZE, W2S_VIEW_LOG2_ALIGN);
}

/**
 * @extends {Error}
 */
export class W2SError {
    /**
     * @private
     * @param {boolean} spvc_error
     * @param {Error} exception
     */
    constructor(spvc_error = false, exception = undefined) {
        if (spvc_error) {
            this.message = wasi.getStderrString();
            if (this.message.length === 0 && exception) {
                this.message = exception.message;
            }
            return;
        }

        (instance.exports.w2s_take_last_error_message)(view_bucket);
        const err_msg = importString(view_bucket);
        if (err_msg) this.message = err_msg;
    }
}

export class CompilationConfigBuilder {
    /**
     * @readonly
     * @type {number}
     * */
    ptr;

    /**
     * @param {W2STarget} target
     * @param {W2SCapabilities} capabilities
     * @param {string[]} extensions
     * @param {W2SAddressingModel} addressing_model
     * @param {W2SMemoryModel} memory_model
     */
    constructor(target, capabilities, extensions, addressing_model, memory_model) {
        let targetArg = 0;

        const ptr = (instance.exports.w2s_config_builder_new)();
        if (ptr === NULLPTR) throw new W2SError()
        registry.register(this, () => (instance.exports.w2s_config_builder_destroy)(ptr))
    }

    /**
     * Drops the `CompilationConfigBuilder` manually, instead of relying on the JavaScript garbage collector.
     */
    destroy() {
        registry.unregister(this)
        (instance.exports.w2s_config_builder_destroy)(this.ptr)
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
    destroy() {
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

        let ptr;
        try {
            ptr = (instance.exports.w2s_compilation_new)(usedConfig, usedBytes.byteOffset, usedBytes.byteLength);
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
     * Returns the text representation of the resulting SPIR-V.
     * @returns {string}
     */
    assembly() {
        (instance.exports.w2s_compilation_assembly)(view_bucket, this.ptr)
        const str =  importString(view_bucket);
        if (str) return str
        throw new W2SError()
    }

    /**
     * Returns a copy of the words that form the resulting SPIR-V.
     * @returns {Uint32Array}
     */
    words() {
        const words = this.wordView;
        const result = new Uint32Array(words.length)
        result.set(words)
        return result
    }

    /**
     * Returns a copy of the bytes that form the resulting SPIR-V.
     * @returns {Uint8Array}
     */
    bytes() {
        const bytes = this.byteView;
        const result = new Uint8Array(bytes.length)
        result.set(bytes)
        return result
    }

    /**
     * Returns a translation of the resulting SPIR-V into GLSL.
     * @returns {string}
     */
    glsl() {
        try {
            (instance.exports.w2s_compilation_glsl)(view_bucket, this.ptr)
        } catch (e) {
            throw new W2SError(true, e);
        }

        const str =  importString(view_bucket);
        if (str) return str
        throw new W2SError()
    }


    /**
     * Returns a translation of the resulting SPIR-V into HLSL.
     * @returns {string}
     */
    hlsl() {
        (instance.exports.w2s_compilation_hlsl)(view_bucket, this.ptr)
        const str =  importString(view_bucket);
        if (str) return str
        throw new W2SError()
    }

    /**
     * Returns a translation of the resulting SPIR-V into MSL.
     * @returns {string}
     */
    msl() {
        (instance.exports.w2s_compilation_msl)(view_bucket, this.ptr)
        const str =  importString(view_bucket);
        if (str) return str
        throw new W2SError()
    }

    /**
     * Returns a translation of the resulting SPIR-V into WGSL.
     * @returns {string}
     */
    wgsl() {
        (instance.exports.w2s_compilation_wgsl)(view_bucket, this.ptr)
        const str =  importString(view_bucket);
        if (str) return str
        throw new W2SError()
    }

    /**
     * View into the WebAssembly memory holding the resulting SPIR-V bytes.
     * @returns {Uint8Array}
     */
    get byteView() {
        (instance.exports.w2s_compilation_bytes)(view_bucket, this.ptr)
        return getByteView(view_bucket)
    }

    /**
     * View into the WebAssembly memory holding the resulting SPIR-V words.
     * @returns {Uint32Array}
     */
    get wordView() {
        (instance.exports.w2s_compilation_words)(view_bucket, this.ptr)
        return getWordView(view_bucket)
    }

    /**
     * Drops the `Compilation` manually, instead of relying on the JavaScript garbage collector.
     */
    destroy() {
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
    const resultView = new DataView(memory.buffer, ptr, W2S_VIEW_SIZE)
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

/**
 * @param {number} ptr Pointer to `w2s_byte_view`
 * @returns {Uint8Array}
 */
function getByteView(ptr) {
    const resultView = new DataView(memory.buffer, ptr, W2S_VIEW_SIZE)
    const strPtr = resultView.getInt32(0, true)
    const strLen = resultView.getInt32(PTR_SIZE, true)

    console.assert(strPtr !== NULLPTR)
    return new Uint8Array(memory.buffer, strPtr, strLen)
}

/**
 * @param {number} ptr Pointer to `w2s_word_view`
 * @returns {Uint32Array}
 */
function getWordView(ptr) {
    const resultView = new DataView(memory.buffer, ptr, W2S_VIEW_SIZE)
    const strPtr = resultView.getInt32(0, true)
    const strLen = resultView.getInt32(PTR_SIZE, true)

    console.assert(strPtr !== NULLPTR)
    return new Uint32Array(memory.buffer, strPtr, strLen / Uint32Array.BYTES_PER_ELEMENT)
}
