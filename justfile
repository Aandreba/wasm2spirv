set windows-shell := ["powershell.exe", "-c"]
export RUST_BACKTRACE := "full"

clean:
    cargo clean
    rm -rf examples/out/*

doc:
    cargo +nightly rustdoc --lib --open --features="khronos-all naga-wgsl" -- --cfg docsrs

cli COMPILER *ARGS:
    cargo run --bin wasm2spirv --features="cli {{COMPILER}}" -- {{ARGS}}

test TEST *ARGS:
    zig build-lib examples/{{TEST}}/{{TEST}}.zig -target wasm32-freestanding -O ReleaseSmall -femit-bin=examples/out/{{TEST}}.wasm -dynamic -rdynamic
    just cli khronos-all examples/out/{{TEST}}.wasm --from-json examples/{{TEST}}/{{TEST}}.json -o examples/out/{{TEST}}.spv {{ARGS}}

test-wat TEST *ARGS:
    just cli examples/{{TEST}}/{{TEST}}.wat --from-json examples/{{TEST}}/{{TEST}}.json -o examples/out/{{TEST}}.spv {{ARGS}}

test-publish *ARGS:
    cargo publish --dry-run --allow-dirty {{ARGS}}
