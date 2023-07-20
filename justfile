set windows-shell := ["powershell.exe", "-c"]
export RUST_BACKTRACE := "full"

clean:
    cargo clean
    rf -rf examples/out/*

doc:
    cargo +nightly rustdoc --lib --open --all-features -- --cfg docsrs

cli *ARGS:
    cargo run --bin wasm2spirv --features clap,color-eyre,serde_json,spirv-tools,spirv_cross -- {{ARGS}}

test-saxpy:
    zig build-lib examples/saxpy.zig -target wasm32-freestanding -O ReleaseSmall -femit-bin=examples/out/saxpy.wasm -dynamic -rdynamic
    just cli examples/out/saxpy.wasm --from-json examples/saxpy.json -o examples/out/saxpy.spv --optimize --validate --show-msl
