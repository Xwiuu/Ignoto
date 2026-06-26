# Multi-stage Dockerfile to build and run the ignoto-node Substrate node on Linux
FROM rust:slim AS builder

ENV CARGO_NET_RETRY=10
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
ENV RUSTC_BOOTSTRAP=1
ENV CARGO_UNSTABLE_JSON_TARGET_SPEC=true


# Install system compilation dependencies
RUN apt-get update -o Acquire::Retries=5 && apt-get install -y -o Acquire::Retries=5 \
    clang \
    libclang-dev \
    llvm-dev \
    cmake \
    protobuf-compiler \
    git \
    make \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown && \
    rustup component add rust-src

WORKDIR /app

# Copy the Cargo configuration and source files
COPY Cargo.toml Cargo.lock ./
COPY circuits/ ./circuits/
COPY node/ ./node/
COPY pallets/ ./pallets/

# Compile the Substrate node in release mode
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    rm -rf /app/target/release/wbuild && \
    (cargo build --release -p ignoto-node || (cat /app/target/release/wbuild/ignoto-runtime/Cargo.toml && exit 1))

# Runner stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/ignoto-node /app/ignoto-node

# Expose RPC port (Substrate default JSON-RPC port)
EXPOSE 9944

# Run the node in development mode by default
ENTRYPOINT ["/app/ignoto-node"]
CMD ["--dev", "--tmp", "--rpc-external"]
