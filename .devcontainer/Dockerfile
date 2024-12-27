FROM debian:12

RUN apt-get update && apt-get install -y \
    curl git zsh xz-utils \
    clang sudo nasm linux-perf && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

RUN groupadd -g 1000 jpeg2png && \
    useradd -m -u 1000 -g jpeg2png jpeg2png && \
    echo "jpeg2png ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/jpeg2png && \
    mkdir /usr/local/cargo && chown -R jpeg2png /usr/local/cargo

USER jpeg2png

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal -c clippy rustfmt

RUN sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended

ENV CARGO_HOME=/usr/local/cargo