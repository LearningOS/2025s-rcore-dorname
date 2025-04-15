# This Dockerfile is adapted from https://github.com/LearningOS/rCore-Tutorial-v3/blob/main/Dockerfile
# with the following major updates:
# - ubuntu 18.04 -> 20.04
# - qemu 5.0.0 -> 7.0.0
# - Extensive comments linking to relevant documentation
FROM ubuntu:20.04

ARG QEMU_VERSION=7.0.0
ARG HOME=/root

# # 配置国内镜像源
# RUN sed -i 's/archive.ubuntu.com/mirrors.ustc.edu.cn/g' /etc/apt/sources.list && \
#     sed -i 's/security.ubuntu.com/mirrors.ustc.edu.cn/g' /etc/apt/sources.list

# 0. Install general tools
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update && \
    apt-get install -y \
        curl \
        git \
        python3 \
        wget \
        sudo \
        gdb \
        make \
        gcc \
        g++ \
        vim \
        tmux

# 1. Set up QEMU RISC-V
WORKDIR ${HOME}
RUN wget https://download.qemu.org/qemu-${QEMU_VERSION}.tar.xz && \
    tar xvJf qemu-${QEMU_VERSION}.tar.xz

RUN apt-get install -y \
        autoconf automake autotools-dev curl libmpc-dev libmpfr-dev libgmp-dev \
        gawk build-essential bison flex texinfo gperf libtool patchutils bc \
        zlib1g-dev libexpat-dev git \
        ninja-build pkg-config libglib2.0-dev libpixman-1-dev libsdl2-dev

WORKDIR ${HOME}/qemu-${QEMU_VERSION}
RUN ./configure --target-list=riscv64-softmmu,riscv64-linux-user && \
    make -j$(nproc) && \
    make install

WORKDIR ${HOME}
RUN rm -rf qemu-${QEMU_VERSION} qemu-${QEMU_VERSION}.tar.xz

RUN qemu-system-riscv64 --version && \
    qemu-riscv64 --version

# 2. Set up Rust
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_VERSION=nightly
RUN set -eux; \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o rustup-init; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --profile minimal --default-toolchain $RUST_VERSION; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME;

RUN rustup --version && \
    cargo --version && \
    rustc --version

# 3. Build env for labs
RUN rustup target add riscv64gc-unknown-none-elf && \
    cargo install cargo-binutils --vers ~0.2 && \
    rustup component add rust-src && \
    rustup component add llvm-tools-preview

# 4. Create workspace directory
RUN mkdir -p /workspace
WORKDIR /workspace

# 5. Set up permissions
RUN chmod -R 777 /workspace 