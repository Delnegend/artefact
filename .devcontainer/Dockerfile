FROM debian:12

RUN apt-get update && apt-get install -y \
    curl git zsh xz-utils \
    clang sudo linux-perf && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

RUN groupadd -g 1000 artefact && \
    useradd -m -u 1000 -g artefact artefact && \
    echo "artefact ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/artefact && \
    mkdir -p /usr/local/cargo && chown -R artefact /usr/local/cargo && \
    mkdir -p /home/artefact/.pnpm-store && chown -R artefact /home/artefact/.pnpm-store

USER artefact
