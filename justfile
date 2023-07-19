set windows-shell := ["powershell.exe", "-c"]
export RUST_BACKTRACE := "full"

test:
    cd examples && cargo test -- --nocapture
    spirv-val examples/out/test.spv

test-to-wgsl: test


test-to-msl: test
    spirv-cross -V --msl --msl-version 20200 --version 330 --no-es examples/out/test.spv --output examples/out/test.comp
