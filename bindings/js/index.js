/**
 * @typedef {("logical" | "physical" | "physical_storage_buffer")} AddressingModel
 * @typedef {("universal" | "vulkan")} TargetPlatform
 * @typedef {("static" | "dynamic")} CapabilityModel
 * @typedef {("hard" | "soft")} MemoryGrowErrorKind
 *
 * @typedef {object} VersionObject
 * @property {number} major
 * @property {number} minor
 *
 * @typedef {object} WasmFeatures
 * @property {boolean} [memory64]
 * @property {boolean} [saturating_float_to_int]
 */

import { init as wasiInit, WASI } from '@wasmer/wasi';
import { integerCapability, integerExecutionMode, integerMemoryModel, stringCapability } from "./spirv.js"

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
 * Initializes the wasm2spirv API.
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
 * @class
 * @extends {Error}
 */
export class W2SError {
    /**
     * @private
     * @param {boolean} spvc_error
     * @param {Error} exception
     */
    constructor(spvc_error = false, exception = undefined) {
        const stdout = wasi.getStdoutString();
        if (stdout.length > 0) {
            console.warn(stdout)
        }

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

/**
 * @class
 */
export class CompilationConfigBuilder {
    /**
     * @readonly
     * @type {number}
     * */
    ptr;

    /**
     * @private
     * @type {number | null}
     */
    static wasm_features_bucket = null;

    /**
     * @param {Target} target
     * @param {Capabilities} capabilities
     * @param {(string[] | undefined)} extensions
     * @param {AddressingModel} addressing_model
     * @param {import("./spirv.js").MemoryModel} memory_model
     */
    constructor(target, capabilities, extensions, addressing_model, memory_model) {
        let addressingModelArg;
        switch (addressing_model) {
            case 'logical':
                addressingModelArg = 0;
                break;
            case 'physical':
                addressingModelArg = 1;
                break;
            case 'physical_storage_buffer':
                addressingModelArg = 2;
                break;
            default:
                throw new Error("Unknown addressing model")
        }

        let memoryModelArg = integerMemoryModel(memory_model)
        extensions ??= []

        // Allocate extension views
        let extensionByteLength = extensions.length * W2S_VIEW_SIZE;
        let extensionArg = (instance.exports.w2s_malloc)(extensionByteLength, W2S_VIEW_LOG2_ALIGN);

        try {
            // Initialize extension views
            const extensionsView = new DataView(memory.buffer, extensionArg)
            /** @type {Uint8Array[]} */
            let buffersToDrop = new Array(extensions.length)
            let i = 0;
            try {
                while (i < extensions.length) {
                    const delta = W2S_VIEW_SIZE * i;
                    const [buffer, len] = exportString(extensions[i])

                    extensionsView.setUint32(delta, buffer.byteOffset);
                    extensionsView.setUint32(delta + 4, len);
                    buffersToDrop[i] = buffer;
                    i++;
                }
            } catch (e) {
                // Deallocate already initialized strings
                for (let j = 0; j < i; j++) {
                    let buffer = buffersToDrop[j];
                    (instance.exports.w2s_free)(buffer.byteOffset, buffer.byteLength, 0)
                }
                throw e;
            }

            try {
                const ptr = (instance.exports.w2s_config_builder_new)(
                    target.ptr,
                    capabilities.ptr,
                    extensionArg,
                    extensions.length,
                    addressingModelArg,
                    memoryModelArg
                );

                if (ptr === NULLPTR) throw new W2SError()
                registry.register(this, () => (instance.exports.w2s_config_builder_destroy)(ptr))
                this.ptr = ptr;
            } finally {
                // Deallocate strings
                for (let buffer of buffersToDrop) {
                    (instance.exports.w2s_free)(buffer.byteOffset, buffer.byteLength, 0)
                }
            }
        } finally {
            (instance.exports.w2s_free)(extensionArg, extensionByteLength, W2S_VIEW_LOG2_ALIGN)
        }
    }

    /**
     * @param {MemoryGrowErrorKind} value
     */
    set memoryGrowErrorKind(value) {
        (instance.exports.w2s_config_builder_set_wasm_features)(this.ptr, value === "hard" ? 0 : 1);
    }

    /**
     * @param {WasmFeatures} value
     */
    set wasmFeatures(value) {
        if (CompilationConfigBuilder.wasm_features_bucket == null) {
            CompilationConfigBuilder.wasm_features_bucket = (instance.exports.w2s_malloc)(2, 0);
        }

        const view = new DataView(memory.buffer, CompilationConfigBuilder.wasm_features_bucket);
        view.setUint8(0, value.memory64 ? 1 : 0);
        view.setUint8(1, value.saturating_float_to_int ? 1 : 0);

        (instance.exports.w2s_config_builder_set_wasm_features)(this.ptr, CompilationConfigBuilder.wasm_features_bucket)
    }

    /**
     * @param {MemoryGrowErrorKind} value
     * @returns {CompilationConfigBuilder}
     */
    setMemoryGrowErrorKind(value) {
        this.memoryGrowErrorKind = value;
        return this;
    }

    /**
     * @param {WasmFeatures} value
     * @returns {CompilationConfigBuilder}
     */
    setWasmFeatures(value) {
        this.wasmFeatures = value;
        return this;
    }

    /**
     * @returns {CompilationConfig}
     */
    build() {
        const builder = (instance.exports.w2s_config_builder_clone)(this.ptr);
        const ptr = (instance.exports.w2s_config_builder_build)(builder);
        return new CompilationConfig(ptr)
    }

    /**
     * @returns {CompilationConfig}
     */
    buildAndDestroy() {
        registry.unregister(this);
        const ptr = (instance.exports.w2s_config_builder_build)(this.ptr);
        return new CompilationConfig(ptr)
    }

    /**
     * Drops the `CompilationConfigBuilder` manually, instead of relying on the JavaScript garbage collector.
     */
    destroy() {
        registry.unregister(this);
        (instance.exports.w2s_config_builder_destroy)(this.ptr);
    }
}

/**
 * @class
 */
export class FunctionConfigBuilder {
    /**
     * @readonly
     * @type {number}
     * */
    ptr;

    constructor() {
        const ptr = (instance.exports.w2s_function_config_builder_new)();
        this.ptr = ptr;
        registry.register(this, () => (instance.exports.w2s_function_config_builder_destroy)(ptr))
    }

    /**
     * @param {import('./spirv.js').ExecutionMode} mode
     * @param {...number} values
     * @returns {FunctionConfigBuilder}
     */
    addExecutionMode(mode, ...values) {
        const data = new Uint32Array(values)
        const modeArg = integerExecutionMode(mode)
        const dataArg = copyWords(data)

        try {
            const res = (w2s_function_config_builder_add_execution_mode)(this.ptr, modeArg, dataArg.byteOffset, dataArg.byteLength);
            if (res === 0) throw new W2SError()
        } finally {
            (instance.exports.w2s_free)(dataArg.byteOffset, dataArg.byteLength, 2);
        }

        return this;
    }

    /**
     * @returns {FunctionConfig}
     */
    build() {
        const clone = (instance.exports.w2s_function_config_builder_clone)(this.ptr);
        const ptr = (instance.exports.w2s_function_config_builder_build)(clone);
        return new FunctionConfig(ptr)
    }

    /**
     * @returns {FunctionConfig}
     */
    buildAndDestroy() {
        registry.unregister(this);
        const ptr = (instance.exports.w2s_function_config_builder_build)(this.ptr);
        return new FunctionConfig(ptr)
    }

    /**
     * Drops the `FunctionConfigBuilder` manually, instead of relying on the JavaScript garbage collector.
     */
    destroy() {
        registry.unregister(this);
        (instance.exports.w2s_function_config_builder_destroy)(this.ptr);
    }
}

/**
 * @class
 */
export class FunctionConfig {
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
        registry.register(this, () => (instance.exports.w2s_function_config_destroy)(ptr))
    }

    /**
     * Drops the `FunctionConfig` manually, instead of relying on the JavaScript garbage collector.
     */
    destroy() {
        registry.unregister(this);
        (instance.exports.w2s_config_destroy)(this.ptr)
    }
}

/**
 * @class
 */
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
        registry.unregister(this);
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

/**
 * @class
 */
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
        const str = importString(view_bucket);
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

        const str = importString(view_bucket);
        if (str) return str
        throw new W2SError()
    }

    /**
     * Returns a translation of the resulting SPIR-V into HLSL.
     * @returns {string}
     */
    hlsl() {
        try {
            (instance.exports.w2s_compilation_hlsl)(view_bucket, this.ptr)
        } catch (e) {
            throw new W2SError(true, e);
        }

        const str = importString(view_bucket);
        if (str) return str
        throw new W2SError()
    }

    /**
     * Returns a translation of the resulting SPIR-V into MSL.
     * @returns {string}
     */
    msl() {
        try {
            (instance.exports.w2s_compilation_msl)(view_bucket, this.ptr)
        } catch (e) {
            throw new W2SError(true, e);
        }

        const str = importString(view_bucket);
        if (str) return str
        throw new W2SError()
    }

    /**
     * Returns a translation of the resulting SPIR-V into WGSL.
     * @returns {string}
     */
    wgsl() {
        try {
            (instance.exports.w2s_compilation_wgsl)(view_bucket, this.ptr)
        } catch (e) {
            throw new W2SError(true, e);
        }

        const str = importString(view_bucket);
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
        registry.unregister(this);
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
 * Clones words into WebAssembly memory.
 * @param {Uint32Array} words
 * @returns {Uint32Array}
 */
function copyWords(words) {
    const newPtr = (instance.exports.w2s_malloc)(words.byteLength, 0);
    const newBytes = new Uint32Array(memory.buffer, newPtr, words.byteLength);

    try {
        newBytes.set(words, 0);
    } catch (e) {
        (instance.exports.w2s_free)(newPtr, words.byteLength, 0);
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

/**
 * @class
 */
export class Target {
    /**
     * @readonly
     * @type {number}
     */
    ptr

    /**
     * @constant
     * @type {number}
     */
    static SIZE = 8;

    /**
     * @constant
     * @type {number}
     */
    static LOG2_ALIGN = 2;

    /**
     * @param {number} ptr
     */
    constructor(ptr) {
        this.ptr = ptr;
    }

    /**
     * Platform where the resulting compilation will run on.
     * @type {TargetPlatform}
     */
    get platform() {
        switch (this.getDataView().getInt32(0, true)) {
            case 0:
                return "universal"
            case 1:
                return "vulkan"
            default:
                throw new Error("Unknown target platform")
        }
    }

    /**
     * Version for the specified {@link platform}
     * @type {Version}
     */
    get version() {
        return new Version(this.ptr + 4)
    }

    set platform(value) {
        let result;
        switch (value) {
            case "universal":
                result = 0;
                break;
            case "vulkan":
                result = 1;
                break;
            default:
                throw new Error("Unknown target platform")
        }

        this.getDataView().setInt32(0, result, true)
    }

    set version(value) {
        const src = new Uint8Array(memory.buffer, value.ptr, Version.SIZE);
        const dst = new Uint8Array(memory.buffer, this.ptr + 4, Version.SIZE);
        dst.set(src);
    }

    /**
     * @param {TargetPlatform} platform
     * @param {VersionObject} version
     * @returns {Target}
     */
    static create(platform, version) {
        let result = Target.alloc();
        result.platform = platform;

        if (version instanceof Version) {
            result.version = version;
        } else {
            const view = new DataView(memory.buffer, result.ptr + 4);
            view.setUint8(0, version.major)
            view.setUint8(1, version.minor)
        }

        return result
    }

    /**
     * Returns an uninitialized `W2STarget`
     * @returns {Target}
     */
    static alloc() {
        const ptr = (instance.exports.w2s_malloc)(Target.SIZE, Target.LOG2_ALIGN);
        const result = new Target(ptr)
        registry.register(result, () => (instance.exports.w2s_free)(ptr, Target.SIZE, Target.LOG2_ALIGN))
        return result
    }

    /**
     * @private
     * @returns {DataView}
     */
    getDataView() {
        return new DataView(memory.buffer, this.ptr)
    }
}

/**
 * @class
 */
export class Capabilities {
    /**
     * @readonly
     * @type {number}
     */
    ptr

    /**
     * @constant
     * @type {number}
     */
    static SIZE = 12;

    /**
     * @constant
     * @type {number}
     */
    static LOG2_ALIGN = 2;

    /**
     * @param {number} ptr
     */
    constructor(ptr) {
        this.ptr = ptr;
    }

    /**
     * @type {CapabilityModel}
     */
    get model() {
        switch (this.getDataView().getInt32(0, true)) {
            case 0:
                return "static"
            case 1:
                return "dynamic"
            default:
                throw new Error("Unknown capability model")
        }
    }

    /**
     * @type {import("./spirv.js").Capability[]}
     */
    get capabilities() {
        const view = this.getDataView();
        const ptr = view.getUint32(4, true)
        const len = view.getUint32(8, true)

        if (ptr === NULLPTR) return []
        const words = new Uint32Array(memory.buffer, ptr, len)
        let result = new Array(len)

        for (let i = 0; i < len; i++) {
            result[i] = stringCapability(words[i])
        }

        return result
    }

    set model(value) {
        let result;
        switch (value) {
            case "static":
                result = 0;
                break;
            case "dynamic":
                result = 1;
                break;
            default:
                throw new Error("Unknown capability model")
        }

        this.getDataView().setInt32(0, result, true)
    }

    set capabilities(value) {
        const view = this.getDataView();
        let ptr = view.getUint32(4, true)
        let len = view.getUint32(8, true)

        if (ptr === NULLPTR || len !== value.length) {
            if (ptr !== NULLPTR) (instance.exports.w2s_free)(ptr, Uint32Array.BYTES_PER_ELEMENT * len, 2);
            ptr = (instance.exports.w2s_malloc)(Uint32Array.BYTES_PER_ELEMENT * value.length, 2);
            len = value.length;

            view.setUint32(4, ptr, true);
            view.setUint32(8, len, true);
        }

        const slice = new Uint32Array(memory.buffer, ptr, len);
        for (let i = 0; i < value.length; i++) {
            slice[i] = integerCapability(value[i])
        }
    }

    /**
     * @param {CapabilityModel} model
     * @param {import("./spirv.js").Capability[]} capabilities
     * @returns {Capabilities}
     */
    static create(model, capabilities = undefined) {
        let result = Capabilities.alloc();
        result.getDataView().setUint32(4, NULLPTR, true); // set capabilities pointer to null

        result.model = model;
        if (capabilities) result.capabilities = capabilities;
        return result
    }

    /**
     * Returns an uninitialized `Capabilities`
     * @returns {Capabilities}
     */
    static alloc() {
        const ptr = (instance.exports.w2s_malloc)(Capabilities.SIZE, Capabilities.LOG2_ALIGN);
        const result = new Capabilities(ptr)
        registry.register(result, () => {
            const view = new DataView(memory.buffer, ptr)
            const capabilitiesPtr = view.getUint32(4, true)
            const capabilitiesLen = view.getUint32(8, true)

            if (capabilitiesPtr !== NULLPTR) {
                (instance.exports.w2s_free)(
                    capabilitiesPtr,
                    Uint32Array.BYTES_PER_ELEMENT * capabilitiesLen,
                    2
                );
            }
            (instance.exports.w2s_free)(ptr, Capabilities.SIZE, Capabilities.LOG2_ALIGN);
        })
        return result
    }

    /**
     * @private
     * @returns {DataView}
     */
    getDataView() {
        return new DataView(memory.buffer, this.ptr)
    }
}

/**
 * Semantic version
 * @class
 */
export class Version {
    /**
     * @readonly
     * @type {number}
     */
    ptr

    /**
     * @constant
     * @type {number}
     */
    static SIZE = 2;

    /**
     * @constant
     * @type {number}
     */
    static LOG2_ALIGN = 0;

    /**
     * @type {number}
     */
    constructor(ptr) {
        this.ptr = ptr
    }

    /**
     * Major version
     * @type {number}
     */
    get major() {
        return this.getDataView().getUint8(0)
    }

    /**
     * Minor version
     * @type {number}
     */
    get minor() {
        return this.getDataView().getUint8(1)
    }

    set major(value) {
        this.getDataView().setUint8(0, value)
    }

    set minor(value) {
        this.getDataView().setUint8(1, value)
    }

    /**
     * @private
     * @returns {DataView}
     */
    getDataView() {
        return new DataView(memory.buffer, this.ptr)
    }
}
