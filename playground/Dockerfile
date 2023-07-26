FROM rust:1.71

WORKDIR /wasm2spirv-playground
COPY . .
WORKDIR playground

# Install zig
RUN apt update && apt install -y jq
RUN "./installers/zig.sh"

# Install playground server
RUN cargo install --path .

# Start playground server
EXPOSE 8080
CMD ["playground"]
