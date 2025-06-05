FROM mcr.microsoft.com/vscode/devcontainers/base:ubuntu-22.04

# Install Rust globally so all users have access
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y \
    && chmod -R a+w $RUSTUP_HOME $CARGO_HOME

# Install system dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    protobuf-compiler \
    pkg-config \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*
