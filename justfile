set windows-shell := ["powershell.exe", "-c"]
export RUST_BACKTRACE := "full"

cli *ARGS:
    cargo run --bin wasm2spirv --features clap,color-eyre,serde_json -- {{ARGS}}

test:
    zig build-lib examples/saxpy.zig -target wasm32-freestanding -O ReleaseSmall -femit-bin=examples/out/saxpy.wasm -dynamic -rdynamic
    cd examples && cargo test -- --nocapture
    spirv-val --target-env vulkan1.1 examples/out/test.spv
    spirv-opt --target-env=vulkan1.1 examples/out/test.spv -o examples/out/test_opt.spv

test-to-wgsl: test
    tint examples/out/test.spv

test-to-msl: test
    spirv-cross -V --msl --msl-version 20200 --no-es examples/out/test.spv --output examples/out/test.msl
    spirv-cross -V --msl --msl-version 20200 --no-es examples/out/test_opt.spv --output examples/out/test_opt.msl
