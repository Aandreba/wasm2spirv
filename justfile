set windows-shell := ["powershell.exe", "-c"]
export RUST_BACKTRACE := "full"

PLAYGROUND_NAME := "wasm2spirv-playground"

clean:
    cargo clean
    rm -rf examples/out/*

doc:
    cargo +nightly rustdoc --lib --open --all-features -- --cfg docsrs

cli *ARGS:
    cargo run --bin wasm2spirv --all-features -- {{ARGS}}

test TEST *ARGS:
    zig build-lib examples/{{TEST}}/{{TEST}}.zig -target wasm32-freestanding -O ReleaseSmall -femit-bin=examples/out/{{TEST}}.wasm -dynamic -rdynamic
    just cli khronos-all examples/out/{{TEST}}.wasm --from-json examples/{{TEST}}/{{TEST}}.json -o examples/out/{{TEST}}.spv {{ARGS}}

test-wat TEST *ARGS:
    just cli khronos-all examples/{{TEST}}/{{TEST}}.wat --from-json examples/{{TEST}}/{{TEST}}.json -o examples/out/{{TEST}}.spv {{ARGS}}

test-publish *ARGS:
    cargo publish --dry-run --allow-dirty {{ARGS}}

playground:
    docker build --tag {{PLAYGROUND_NAME}} {{justfile_directory()}}/playground
    docker run {{PLAYGROUND_NAME}}
