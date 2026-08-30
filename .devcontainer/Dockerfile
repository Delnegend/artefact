FROM debian:13-slim

ARG MOLD_VERSION=2.40.4

ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8
ENV SHELL=/bin/bash

RUN --mount=type=cache,target=/var/cache/apt \
    apt-get update && apt-get install -y --no-install-recommends \
    build-essential ca-certificates curl git just fzf \
    clang sudo unzip zip xz-utils linux-perf pkg-config && \
    rm -rf /var/lib/apt/lists/*

RUN groupadd -g 1000 artefact && \
    useradd -m -u 1000 -g artefact -s /bin/bash artefact && \
    echo "artefact ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/artefact && \
    chmod 0440 /etc/sudoers.d/artefact && \
    chsh -s /bin/bash artefact && \
    mkdir -p /usr/local/cargo && chown -R artefact:artefact /usr/local/cargo

SHELL ["/bin/bash", "-c"]
USER artefact

ENV CARGO_HOME=/usr/local/cargo
ENV PATH="/usr/local/cargo/bin:/home/artefact/.bun/bin:${PATH}"

# Rust toolchain (nightly, minimal) + rust-analyzer — pre-installed so container
# works even if postCreateCommand is skipped (e.g. Zed)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly --profile minimal --no-modify-path && \
    /usr/local/cargo/bin/rustup component add rust-analyzer

# mold — pinned to keep linker fast/reproducible; also configure cargo to use it
RUN curl -fsSL -o /tmp/mold.tar.gz "https://github.com/rui314/mold/releases/download/v${MOLD_VERSION}/mold-${MOLD_VERSION}-x86_64-linux.tar.gz" && \
    echo "4c999e19ffa31afa5aa429c679b665d5e2ca5a6b6832ad4b79668e8dcf3d8ec1  /tmp/mold.tar.gz" | sha256sum -c - && \
    tar -xf /tmp/mold.tar.gz -C /usr/local/cargo "mold-${MOLD_VERSION}-x86_64-linux" && \
    rm /tmp/mold.tar.gz && \
    printf '[target.x86_64-unknown-linux-gnu]\nlinker = "clang"\nrustflags = ["-C", "link-arg=-fuse-ld=/usr/local/cargo/mold-%s-x86_64-linux/bin/mold"]\n' "$MOLD_VERSION" > /usr/local/cargo/config.toml

# cargo-binstall + flamegraph + wasm-pack (cached in image, not postinstall)
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash && \
    cargo binstall -y flamegraph wasm-pack

# bun — pre-installed for cache + non-interactive shells
RUN curl -fsSL https://bun.sh/install | bash && sudo ln -sf /home/artefact/.bun/bin/bun /usr/local/bin/bun

# just completions for interactive shells (was in postinstall.sh, now in image for cache)
RUN echo 'eval "$(just --completions bash)"' >> /home/artefact/.bashrc

# Ensure bun/cargo are in PATH for non-interactive shells (Zed, `just build`)
ENV PATH="/home/artefact/.bun/bin:/usr/local/cargo/bin:${PATH}"
