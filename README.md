[![Crates.io](https://img.shields.io/crates/v/wasm2spirv)](https://crates.io/crates/wasm2spirv)
[![docs.rs](https://img.shields.io/docsrs/wasm2spirv)](https://docs.rs/wasm2spirv/latest)
[![GitHub](https://img.shields.io/github/license/Aandreba/wasm2spirv)](https://github.com/Aandreba/wasm2spirv)

# wasm2spirv - Compile your WebAssembly programs into SPIR-V shaders

> **Warning**
>
> `wasm2spirv` is still in early development, and not production ready.

This repository contains the code for both, the CLI and library for wasm2spirv.

## Installation

To add `wasm2spirv` as a library for your Rust project, run this command on
you'r project's root directory: `cargo add wasm2spirv`

To install the latest version of the `wasm2spirv` CLI, run this command:
`cargo install wasm2spirv`

## Library features

- [`llvm-tools`](https://github.com/EmbarkStudios/spirv-tools-rs) enables
  optimization and validation.
- [`llvm_cross`](https://github.com/grovesNL/spirv_cross) enables
  cross-compilation to GLSL, HLSL and MSL.
