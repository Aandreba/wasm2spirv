set windows-shell := ["powershell.exe", "-c"]
export RUST_BACKTRACE := "full"

test *ARGS:
    cargo test {{ARGS}} -- --nocapture
    spirv-val test.spv --target-env vulkan1.2
