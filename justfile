set windows-shell := ["powershell.exe", "-c"]
export RUST_BACKTRACE := "full"

clean:
    cargo clean
    rf -rf examples/out/*

doc:
    cargo +nightly rustdoc --lib --open --all-features -- --cfg docsrs

cli *ARGS:
    cargo run --bin wasm2spirv --features clap,color-eyre,serde_json,spirv-tools,spirv_cross -- {{ARGS}}

test:
    zig build-lib examples/saxpy.zig -target wasm32-freestanding -O ReleaseSmall -femit-bin=examples/out/saxpy.wasm -dynamic -rdynamic
    just cli examples/saxpy.wat --from-json examples/saxpy.json -o examples/out/saxpy.spv --validate --show-hlsl

test-to-wgsl: test
    tint examples/out/test.spv

test-to-msl: test
    spirv-cross -V --msl --msl-version 20200 --no-es examples/out/test.spv --output examples/out/test.msl
    spirv-cross -V --msl --msl-version 20200 --no-es examples/out/test_opt.spv --output examples/out/test_opt.msl
