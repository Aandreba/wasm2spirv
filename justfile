set windows-shell := ["powershell.exe", "-c"]
export RUST_BACKTRACE := "full"

clean:
    cargo clean
    rf -rf examples/out/*

doc:
    cargo +nightly rustdoc --lib --open --all-features -- --cfg docsrs

cli *ARGS:
    cargo run --bin wasm2spirv --features clap,color-eyre,serde_json,spirv-tools,spirv_cross -- {{ARGS}}

test TEST *ARGS:
    zig build-lib examples/{{TEST}}.zig -target wasm32-freestanding -O ReleaseSmall -femit-bin=examples/out/{{TEST}}.wasm -dynamic -rdynamic
    just cli examples/out/{{TEST}}.wasm --from-json examples/{{TEST}}.json -o examples/out/{{TEST}}.spv {{ARGS}}
