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
- Supports validation and optimization of the resulting SPIR-V via
  [`spirv-tools`](https://github.com/EmbarkStudios/spirv-tools-rs)
- Can be compiled to WebAssembly itself
  - You won't be able to use `spirv-tools` or `spirv_cross` in WebAssembly
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

| Target      | Windows                         | Linux                           | macOS                           | WebAssembly       |
| ----------- | ------------------------------- | ------------------------------- | ------------------------------- | ----------------- |
| SPIR-V      | ✅                              | ✅                              | ✅                              | ✅                |
| GLSL        | ☑️ (spvc-glsl/naga-glsl)         | ☑️ (spvc-glsl/naga-glsl)         | ☑️ (spvc-glsl/naga-glsl)         | ☑️ (naga-glsl)     |
| HLSL        | ☑️ (spvc-hlsl/naga-hlsl)         | ☑️ (spvc-hlsl/naga-hlsl)         | ☑️ (spvc-hlsl/naga-hlsl)         | ☑️ (naga-hlsl)     |
| Metal (MSL) | ☑️ (spvc-msl/naga-msl)           | ☑️ (spvc-msl/naga-msl)           | ☑️ (spvc-msl/naga-msl)           | ☑️ (naga-msl)      |
| WGSL        | ☑️ (naga-wgsl)                   | ☑️ (naga-wgsl)                   | ☑️ (naga-wgsl)                   | ☑️ (naga-wgsl)     |
| DXIL        | ❌                              | ❌                              | ❌                              | ❌                |
| OpenCL C    | ❌                              | ❌                              | ❌                              | ❌                |
| Cuda        | ❌                              | ❌                              | ❌                              | ❌                |
| Validation  | ☑️ (spvt-validate/naga-validate) | ☑️ (spvt-validate/naga-validate) | ☑️ (spvt-validate/naga-validate) | ☑️ (naga-validate) |

- ✅ Supported
- ☑️ Supported, but library requires cargo feature(s)
- ❌ Unsupported

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
  "version": "1.3",
  "addressing_model": "logical",
  "memory_model": "GLSL450",
  "capabilities": { "dynamic": [] },
  "extensions": [],
  "functions": {
    "2": {
      "execution_model": "GLCompute",
      "execution_mode": {
        "local_size": [1, 1, 1]
      },
      "params": {
        "0": {
          "type": {
            "Structured": "i32"
          },
          "kind": {
            "descriptor_set": {
              "storage_class": "StorageBuffer",
              "set": 0,
              "binding": 0
            }
          }
        },
        "1": {
          "type": {
            "Structured": "f32"
          },
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
            "StructuredArray": "f32"
          },
          "kind": {
            "descriptor_set": {
              "storage_class": "StorageBuffer",
              "set": 0,
              "binding": 2
            }
          },
          "is_extern_pointer": true
        },
        "3": {
          "type": {
            "StructuredArray": "f32"
          },
          "kind": {
            "descriptor_set": {
              "storage_class": "StorageBuffer",
              "set": 0,
              "binding": 3
            }
          },
          "is_extern_pointer": true
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
; Bound: 70
OpCapability Shader
OpCapability VariablePointers
OpMemoryModel Logical GLSL450
OpEntryPoint GLCompute %3 "main" %6 %7
OpExecutionMode %3 LocalSize 1 1 1
OpDecorate %6 BuiltIn GlobalInvocationId
OpDecorate %7 BuiltIn NumWorkgroups
OpDecorate %10 Block
OpMemberDecorate %10 0 Offset 0
OpDecorate %12 DescriptorSet 0
OpDecorate %12 Binding 0
OpDecorate %14 Block
OpMemberDecorate %14 0 Offset 0
OpDecorate %16 DescriptorSet 0
OpDecorate %16 Binding 1
OpDecorate %17 ArrayStride 4
OpDecorate %18 Block
OpMemberDecorate %18 0 Offset 0
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
%28 = OpTypePointer Function %19
%32 = OpConstant  %1  2
%41 = OpTypeBool
%43 = OpTypePointer StorageBuffer %1
%44 = OpConstant  %1  0
%49 = OpTypePointer StorageBuffer %13
%51 = OpConstant  %1  4
%3 = OpFunction  %8  None %9
%22 = OpLabel
%48 = OpVariable  %23  Function %44
%29 = OpVariable  %28  Function
%27 = OpVariable  %23  Function
%26 = OpVariable  %23  Function
%25 = OpVariable  %23  Function
%24 = OpVariable  %23  Function
%30 = OpLoad  %4  %6
%31 = OpCompositeExtract  %1  %30 0
OpStore %24 %31
%33 = OpShiftLeftLogical  %1  %31 %32
OpStore %25 %33
%34 = OpLoad  %4  %7
%35 = OpCompositeExtract  %1  %34 0
OpStore %26 %35
%36 = OpShiftLeftLogical  %1  %35 %32
OpStore %27 %36
OpBranch %37
%37 = OpLabel
OpBranch %38
%38 = OpLabel
%42 = OpLoad  %1  %24
%45 = OpAccessChain  %43  %12 %44
%46 = OpLoad  %1  %45
%47 = OpUGreaterThanEqual  %41  %42 %46
OpLoopMerge %39 %40 None
OpBranchConditional %47 %39 %40
%40 = OpLabel
OpStore %29 %21
OpCopyMemory %48 %25
%50 = OpLoad  %1  %25
%52 = OpUDiv  %1  %50 %51
%53 = OpAccessChain  %49  %21 %44 %52
%54 = OpLoad  %19  %29
%55 = OpLoad  %1  %48
%56 = OpUDiv  %1  %55 %51
%57 = OpAccessChain  %49  %54 %44 %56
%58 = OpLoad  %13  %57 Aligned 4
%59 = OpUDiv  %1  %50 %51
%60 = OpAccessChain  %49  %20 %44 %59
%61 = OpLoad  %13  %60 Aligned 4
%62 = OpAccessChain  %49  %16 %44
%63 = OpLoad  %13  %62
%64 = OpFMul  %13  %61 %63
%65 = OpFAdd  %13  %58 %64
OpStore %53 %65 Aligned 4
%66 = OpLoad  %1  %27
%67 = OpIAdd  %1  %50 %66
OpStore %25 %67
%68 = OpLoad  %1  %26
%69 = OpIAdd  %1  %42 %68
OpStore %24 %69
OpBranch %38
%39 = OpLabel
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
    uint _24 = gl_GlobalInvocationID.x;
    uint _33 = gl_GlobalInvocationID.x << 2u;
    uint _25 = _33;
    uint _26 = gl_NumWorkGroups.x;
    uint _36 = gl_NumWorkGroups.x << 2u;
    uint _27 = _36;
    device _18* _29;
    uint _42;
    for (;;)
    {
        _42 = _24;
        if (_42 >= _12._m0)
        {
            break;
        }
        else
        {
            _29 = &_21;
            _48 = _25;
            _21._m0[_25 / 4u] = _29->_m0[_48 / 4u] + (_20._m0[_25 / 4u] * _16._m0);
            _25 += _27;
            _24 = _42 + _26;
            continue;
        }
    }
}
```

## Installation

To add `wasm2spirv` as a library for your Rust project, run this command on
you'r project's root directory: `cargo add wasm2spirv`

To install the latest version of the `wasm2spirv` CLI, run this command:
`cargo install wasm2spirv`

## Cargo features

- [`spirv-tools`](https://github.com/EmbarkStudios/spirv-tools-rs) enables
  optimization and validation.
- [`spirv_cross`](https://github.com/grovesNL/spirv_cross) enables
  cross-compilation to GLSL, HLSL and MSL.

## Related projects

- [SPIRV-LLVM](https://github.com/KhronosGroup/SPIRV-LLVM-Translator) is an
  official Khronos tool to compile LLVM IR into SPIR-V.
- [Wasmer](https://github.com/wasmerio/wasmer) is a WebAssembly runtime that
  runs WebAssembly programs on the host machine.
- [Bytecoder](https://github.com/mirkosertic/Bytecoder) can translate JVM code
  into JavaScript, WebAssembly and OpenCL.
- [Naga](https://github.com/gfx-rs/naga/) is a translator from, and to, various
  shading languages and IRs.
