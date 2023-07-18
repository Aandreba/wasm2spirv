set windows-shell := ["powershell.exe", "-c"]
export RUST_BACKTRACE := "full"

test *ARGS:
    cargo test {{ARGS}} -- --nocapture
    spirv-val test.spv
    spirv-cross -V --msl --msl-version 20200 --version 330 --no-es test.spv --output test.comp
