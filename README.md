[![Crates.io](https://img.shields.io/crates/v/wasm2spirv)](https://crates.io/crates/wasm2spirv)
[![docs.rs](https://img.shields.io/docsrs/wasm2spirv)](https://docs.rs/wasm2spirv/latest)
[![GitHub](https://img.shields.io/github/license/Aandreba/wasm2spirv)](https://github.com/Aandreba/wasm2spirv)

# wasm2spirv - Compile your WebAssembly programs into SPIR-V shaders

> **Warning**
>
> `wasm2spirv` is still in early development, and not production ready.

This repository contains the code for both, the CLI and library for wasm2spirv.
wasm2spirv allows you to compile any WebAssembly program into a SPIR-V shader

## Features

- Compiles your WebAssembly programs into SPIR-V
- Can transpile into other various shading languages
- Supports validation and optimization of the resulting SPIR-V
- Can be compiled to WebAssembly itself
  - You won't be able to use `spirv-tools` or `tree-sitter` in WebAssembly
  - `spirvcross` only works on WASI
  - CLI will have to be compiled to WASI

## Caveats

- Still in early development
  - Unexpected bugs and crashes are to be expected
  - Still working through the WebAssembly MVP
- WebAssembly programs with memory allocations will not work
  - You can customize whether the `memory.grow` instruction errors the
    compilation (hard errors) or always returns -1 (soft errors)
- You'll have to manually provide quite some extra information
  - This is because SPIR-V has a lot of constructs compared to the simplicity of
    WebAssembly.
  - wasm2spirv can do **some** inferrence based on the WebAssembly program
    itself, but it's usually better to specify most the information on the
    configuration.
  - The plan for the future is to be able to store the config information inside
    the WebAssembly program itself.

## Compilation Targets

| Target      | Windows                         | Linux                           | macOS                           | WebAssembly              |
| ----------- | ------------------------------- | ------------------------------- | ------------------------------- | ------------------------ |
| SPIR-V      | ✅                              | ✅                              | ✅                              | ✅                       |
| GLSL        | ☑️ (spvc-glsl/naga-glsl)         | ☑️ (spvc-glsl/naga-glsl)         | ☑️ (spvc-glsl/naga-glsl)         | ☑️ (spvc-glsl*/naga-glsl) |
| HLSL        | ☑️ (spvc-hlsl/naga-hlsl)         | ☑️ (spvc-hlsl/naga-hlsl)         | ☑️ (spvc-hlsl/naga-hlsl)         | ☑️ (spvc-hlsl*/naga-hlsl) |
| Metal (MSL) | ☑️ (spvc-msl/naga-msl)           | ☑️ (spvc-msl/naga-msl)           | ☑️ (spvc-msl/naga-msl)           | ☑️ (spvc-msl*/naga-msl)   |
| WGSL        | ☑️ (naga-wgsl)                   | ☑️ (naga-wgsl)                   | ☑️ (naga-wgsl)                   | ☑️ (naga-wgsl)            |
| DXIL        | ❌                              | ❌                              | ❌                              | ❌                       |
| OpenCL C    | ❌                              | ❌                              | ❌                              | ❌                       |
| Cuda        | ❌                              | ❌                              | ❌                              | ❌                       |
| Validation  | ☑️ (spvt-validate/naga-validate) | ☑️ (spvt-validate/naga-validate) | ☑️ (spvt-validate/naga-validate) | ☑️ (naga-validate)        |

- ✅ Supported
- ☑️ Supported, but requires cargo feature(s)
- ❌ Unsupported

\* This feature is only supported on WASI

> **Note**
>
> The CLI programs built by the releases use the Khronos compilers/validators
> whenever possible, faling back to naga compilers/validators if the Khronos are
> not available or are not supported on that platform.

## Examples

You can find a few examples on the "examples" directory, with their Zig file,
translated WebAssembly Text, and compilation configuration file.

### Saxpy example

Zig program

```zig
export fn main(n: usize, alpha: f32, x: [*]const f32, y: [*]f32) void {
    var i = gl_GlobalInvocationID(0);
    const size = gl_NumWorkGroups(0);

    while (i < n) {
        y[i] += alpha * x[i];
        i += size;
    }
}

extern "spir_global" fn gl_GlobalInvocationID(u32) usize;
extern "spir_global" fn gl_NumWorkGroups(u32) usize;
```

WebAssembly text

```wasm
(module
  (type (;0;) (func (param i32) (result i32)))
  (type (;1;) (func (param i32 f32 i32 i32)))
  (import "spir_global" "gl_GlobalInvocationID" (func (;0;) (type 0)))
  (import "spir_global" "gl_NumWorkGroups" (func (;1;) (type 0)))
  (func (;2;) (type 1) (param i32 f32 i32 i32)
    (local i32 i32 i32 i32 i32)
    i32.const 0
    call 0
    local.tee 4
    i32.const 2
    i32.shl
    local.set 5
    i32.const 0
    call 1
    local.tee 6
    i32.const 2
    i32.shl
    local.set 7
    block  ;; label = @1
      loop  ;; label = @2
        local.get 4
        local.get 0
        i32.ge_u
        br_if 1 (;@1;)
        local.get 3
        local.get 5
        i32.add
        local.tee 8
        local.get 8
        f32.load
        local.get 2
        local.get 5
        i32.add
        f32.load
        local.get 1
        f32.mul
        f32.add
        f32.store
        local.get 5
        local.get 7
        i32.add
        local.set 5
        local.get 4
        local.get 6
        i32.add
        local.set 4
        br 0 (;@2;)
      end
    end)
  (memory (;0;) 16)
  (global (;0;) (mut i32) (i32.const 1048576))
  (export "memory" (memory 0))
  (export "main" (func 2)))
```

Configuration file (in JSON)

```json
{
  "platform": {
    "vulkan": "1.1"
  },
  "addressing_model": "logical",
  "memory_model": "GLSL450",
  "capabilities": { "dynamic": ["VariablePointers"] },
  "extensions": ["VH_KHR_variable_pointers"],
  "functions": {
    "2": {
      "execution_model": "GLCompute",
      "execution_modes": [{
        "local_size": [1, 1, 1]
      }],
      "params": {
        "0": {
          "type": "i32",
          "kind": {
            "descriptor_set": {
              "storage_class": "StorageBuffer",
              "set": 0,
              "binding": 0
            }
          }
        },

        "1": {
          "type": "f32",
          "kind": {
            "descriptor_set": {
              "storage_class": "StorageBuffer",
              "set": 0,
              "binding": 1
            }
          }
        },

        "2": {
          "type": {
            "size": "fat",
            "storage_class": "StorageBuffer",
            "pointee": "f32"
          },
          "kind": {
            "descriptor_set": {
              "storage_class": "StorageBuffer",
              "set": 0,
              "binding": 2
            }
          },
          "pointer_size": "fat"
        },

        "3": {
          "type": {
            "size": "fat",
            "storage_class": "StorageBuffer",
            "pointee": "f32"
          },
          "kind": {
            "descriptor_set": {
              "storage_class": "StorageBuffer",
              "set": 0,
              "binding": 3
            }
          }
        }
      }
    }
  }
}
```

SPIR-V result

```asm
; SPIR-V
; Version: 1.3
; Generator: rspirv
; Bound: 73
OpCapability VariablePointers
OpCapability Shader
OpExtension "VH_KHR_variable_pointers"
OpMemoryModel Logical GLSL450
OpEntryPoint GLCompute %3 "main" %6 %7
OpExecutionMode %3 LocalSize 1 1 1
OpDecorate %6 BuiltIn GlobalInvocationId
OpDecorate %7 BuiltIn NumWorkgroups
OpMemberDecorate %10 0 Offset 0
OpDecorate %10 Block
OpDecorate %12 DescriptorSet 0
OpDecorate %12 Binding 0
OpMemberDecorate %14 0 Offset 0
OpDecorate %14 Block
OpDecorate %16 DescriptorSet 0
OpDecorate %16 Binding 1
OpDecorate %17 ArrayStride 4
OpMemberDecorate %18 0 Offset 0
OpDecorate %18 Block
OpDecorate %20 DescriptorSet 0
OpDecorate %20 Binding 2
OpDecorate %21 DescriptorSet 0
OpDecorate %21 Binding 3
%1 = OpTypeInt 32 0
%2 = OpConstant  %1  1048576
%4 = OpTypeVector %1 3
%5 = OpTypePointer Input %4
%6 = OpVariable  %5  Input
%7 = OpVariable  %5  Input
%8 = OpTypeVoid
%9 = OpTypeFunction %8
%10 = OpTypeStruct %1
%11 = OpTypePointer StorageBuffer %10
%12 = OpVariable  %11  StorageBuffer
%13 = OpTypeFloat 32
%14 = OpTypeStruct %13
%15 = OpTypePointer StorageBuffer %14
%16 = OpVariable  %15  StorageBuffer
%17 = OpTypeRuntimeArray %13
%18 = OpTypeStruct %17
%19 = OpTypePointer StorageBuffer %18
%20 = OpVariable  %19  StorageBuffer
%21 = OpVariable  %19  StorageBuffer
%23 = OpTypePointer Function %1
%28 = OpConstant  %1  2
%39 = OpTypeBool
%41 = OpConstant  %1  0
%42 = OpTypePointer StorageBuffer %1
%46 = OpTypePointer Function %19
%50 = OpTypePointer StorageBuffer %13
%51 = OpConstant  %1  4
%3 = OpFunction  %8  None %9
%22 = OpLabel
%48 = OpVariable  %23  Function %41
%47 = OpVariable  %46  Function
%33 = OpVariable  %23  Function
%30 = OpVariable  %23  Function
%27 = OpVariable  %23  Function
%24 = OpVariable  %23  Function
%25 = OpLoad  %4  %6
%26 = OpCompositeExtract  %1  %25 0
OpStore %24 %26
%29 = OpShiftLeftLogical  %1  %26 %28
OpStore %27 %29
%31 = OpLoad  %4  %7
%32 = OpCompositeExtract  %1  %31 0
OpStore %30 %32
%34 = OpShiftLeftLogical  %1  %32 %28
OpStore %33 %34
OpBranch %35
%35 = OpLabel
OpBranch %36
%36 = OpLabel
%40 = OpLoad  %1  %24
%43 = OpAccessChain  %42  %12 %41
%44 = OpLoad  %1  %43
%45 = OpUGreaterThanEqual  %39  %40 %44
OpLoopMerge %37 %38 None
OpBranchConditional %45 %37 %38
%38 = OpLabel
OpStore %47 %21
%49 = OpLoad  %1  %27
OpStore %48 %49
%52 = OpUDiv  %1  %49 %51
%53 = OpAccessChain  %50  %21 %41 %52
%54 = OpLoad  %19  %47
%55 = OpLoad  %1  %48
%56 = OpUDiv  %1  %55 %51
%57 = OpAccessChain  %50  %54 %41 %56
%58 = OpLoad  %13  %57 Aligned 4
%59 = OpLoad  %1  %27
%60 = OpUDiv  %1  %59 %51
%61 = OpAccessChain  %50  %20 %41 %60
%62 = OpLoad  %13  %61 Aligned 4
%63 = OpAccessChain  %50  %16 %41
%64 = OpLoad  %13  %63
%65 = OpFMul  %13  %62 %64
%66 = OpFAdd  %13  %58 %65
OpStore %53 %66 Aligned 4
%67 = OpLoad  %1  %27
%68 = OpLoad  %1  %33
%69 = OpIAdd  %1  %67 %68
OpStore %27 %69
%70 = OpLoad  %1  %24
%71 = OpLoad  %1  %30
%72 = OpIAdd  %1  %70 %71
OpStore %24 %72
OpBranch %36
%37 = OpLabel
OpReturn
OpFunctionEnd
```

Metal translation

```metal
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct _10
{
    uint _m0;
};

struct _14
{
    float _m0;
};

struct _18
{
    float _m0[1];
};

kernel void main0(device _10& _12 [[buffer(0)]], device _14& _16 [[buffer(1)]], device _18& _20 [[buffer(2)]], device _18& _21 [[buffer(3)]], uint3 gl_GlobalInvocationID [[thread_position_in_grid]], uint3 gl_NumWorkGroups [[threadgroups_per_grid]])
{
    uint _48 = 0u;
    uint _29 = gl_GlobalInvocationID.x << 2u;
    uint _34 = gl_NumWorkGroups.x << 2u;
    device _18* _47;
    for (uint _24 = gl_GlobalInvocationID.x, _27 = _29, _30 = gl_NumWorkGroups.x, _33 = _34; !(_24 >= _12._m0); )
    {
        _47 = &_21;
        _48 = _27;
        _21._m0[_27 / 4u] = _47->_m0[_48 / 4u] + (_20._m0[_27 / 4u] * _16._m0);
        _27 += _33;
        _24 += _30;
        continue;
    }
}
```

## Installation

To add `wasm2spirv` as a library for your Rust project, run this command on
you'r project's root directory.\
`cargo add wasm2spirv`

To install the latest version of the `wasm2spirv` CLI, run this command.\
`cargo install wasm2spirv`

## Cargo features

- [`spirv-tools`](https://github.com/EmbarkStudios/spirv-tools-rs) enables
  optimization and validation.
- [`spirvcross`](https://github.com/Aandreba/spirvcross) enables
  cross-compilation to GLSL, HLSL and MSL.
- [`tree-sitter`](https://github.com/tree-sitter/tree-sitter) enables syntax
  highlighting on the CLI.
- [`naga`](https://github.com/gfx-rs/naga/) enables cross-compilation for GLSL,
  HLSL, MSL and WGSL.

## Related projects

- [SPIRV-LLVM](https://github.com/KhronosGroup/SPIRV-LLVM-Translator) is an
  official Khronos tool to compile LLVM IR into SPIR-V.
- [Wasmer](https://github.com/wasmerio/wasmer) is a WebAssembly runtime that
  runs WebAssembly programs on the host machine.
- [Bytecoder](https://github.com/mirkosertic/Bytecoder) can translate JVM code
  into JavaScript, WebAssembly and OpenCL.
- [Naga](https://github.com/gfx-rs/naga/) is a translator from, and to, various
  shading languages and IRs.
