# Development environment for Beryl Router (aarch64 optimized)
# Works natively on Apple Silicon Macs with OrbStack/Docker

FROM rust:1.83-slim-bookworm

# Install dependencies
# - llvm/clang: required for bpf-linker
# - musl-tools: required for static linking (aarch64-unknown-linux-musl)
# - build-essential: gcc and friends
# - git: for cargo
RUN apt-get update && apt-get install -y \
    build-essential \
    clang \
    llvm \
    musl-tools \
    pkg-config \
    libssl-dev \
    git \
    sudo \
    && rm -rf /var/lib/apt/lists/*

# Create development user 'beryl' to match host UID/GID (often 1000:1000)
# This prevents permission issues with mounted volumes
ARG USER=beryl
ARG UID=1000
ARG GID=1000

RUN groupadd -g ${GID} ${USER} \
    && useradd -u ${UID} -g ${GID} -m ${USER} \
    && echo "${USER} ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/${USER}

USER ${USER}
WORKDIR /home/${USER}/project

# Install Rust Nightly (required for aya/eBPF)
RUN rustup toolchain install nightly \
    && rustup default nightly \
    && rustup component add rust-src

# Install bpf-linker
RUN cargo install bpf-linker

# Add musl target for static binary generation
RUN rustup target add aarch64-unknown-linux-musl

# Pre-create cache directories
RUN mkdir -p /home/${USER}/.cargo/registry

CMD ["/bin/bash"]
