FROM mcr.microsoft.com/vscode/devcontainers/base:ubuntu-22.04

# -------------------
# Set environment variables
# -------------------
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    FOUNDRY_HOME=/usr/local/foundry \
    PATH=/usr/local/cargo/bin:/usr/local/foundry/bin:$PATH

# -------------------
# Install Rust
# -------------------
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME

# -------------------
# Install packages
# -------------------
RUN apt-get update && apt-get install -y \
    libssl-dev \
    protobuf-compiler \
    libpq-dev \
    pkg-config \
    llvm-dev \
    libclang-dev \
    clang \
    curl \
    git \
    build-essential \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# -------------------
# Install Foundry safely
# -------------------
RUN curl -L https://foundry.paradigm.xyz | bash && \
    . ~/.bashrc && \
    foundryup && \
    mkdir -p $FOUNDRY_HOME/bin && \
    cp ~/.foundry/bin/* $FOUNDRY_HOME/bin/ && \
    chmod -R a+rx $FOUNDRY_HOME && \
    chown -R vscode:vscode $FOUNDRY_HOME

# -------------------
# Verify
# -------------------
RUN forge --version && cast --version
