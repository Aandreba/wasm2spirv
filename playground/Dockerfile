FROM ubuntu:22.04

WORKDIR /wasm2spirv-playground
COPY . .

# Install Rust
RUN "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"

# Install playground server
RUN cargo install --path .

# Start playground server
