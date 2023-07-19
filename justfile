set windows-shell := ["powershell.exe", "-c"]
export RUST_BACKTRACE := "full"

test:
    zig build-lib examples/saxpy.zig -target wasm32-freestanding -O ReleaseSmall -femit-bin=examples/out/saxpy.wasm -dynamic -rdynamic
    cd examples && cargo test -- --nocapture
    spirv-val --target-env vulkan1.1 examples/out/test.spv

test-to-wgsl: test
    tint examples/out/test.spv

test-to-msl: test
    spirv-cross -V --msl --msl-version 20200 --version 330 --no-es examples/out/test.spv --output examples/out/test.comp
