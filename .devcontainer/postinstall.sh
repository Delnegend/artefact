#!/usr/bin/env bash

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly
. "$HOME/.cargo/env"

# mold
ver="2.40.4"
curl -L -o /tmp/mold.tar.gz https://github.com/rui314/mold/releases/download/v$ver/mold-$ver-x86_64-linux.tar.gz
if [ "$(sha256sum /tmp/mold.tar.gz | awk '{print $1}')" != "4c999e19ffa31afa5aa429c679b665d5e2ca5a6b6832ad4b79668e8dcf3d8ec1" ]; then
    echo "mold tarball checksum failed"
else
    sudo rm -rf /usr/local/cargo/mold*
    sudo tar -xf /tmp/mold.tar.gz -C /usr/local/cargo mold-$ver-x86_64-linux
fi
rm -f /tmp/mold.tar.gz

# configure cargo to use mold
rm -f /usr/local/cargo/config.toml
printf "[target.x86_64-unknown-linux-gnu]\nlinker = \"clang\"\nrustflags = [\"-C\", \"link-arg=-fuse-ld=/usr/local/cargo/mold-$ver-x86_64-linux/bin/mold\"]" > /usr/local/cargo/config.toml

# just
ver="1.45.0"
curl -L -o /tmp/just.tar.gz https://github.com/casey/just/releases/download/$ver/just-$ver-x86_64-unknown-linux-musl.tar.gz
if [ ! "$(sha256sum /tmp/just.tar.gz | awk '{print $1}')" != "dc3f958aaf8c6506dd90426e9b03f86dd15e74a6467ee0e54929f750af3d9e49" ]; then
    echo "just tarball checksum failed"
else
    sudo rm -rf /usr/local/bin/just
    sudo tar -xf /tmp/just.tar.gz -C /usr/local/bin just
fi
rm -f /tmp/just.tar.gz
echo 'alias j=just' >> ~/.bashrc
echo 'eval "$(just --completions bash)"' >> ~/.bashrc
echo 'complete -F _just j' >> ~/.bashrc

# fzf
ver="0.67.0"
curl -L -o /tmp/fzf.tar.gz https://github.com/junegunn/fzf/releases/download/v$ver/fzf-$ver-linux_amd64.tar.gz
if [ "$(sha256sum /tmp/fzf.tar.gz | awk '{print $1}')" != "4be08018ca37b32518c608741933ea335a406de3558242b60619e98f25be2be1" ]; then
    echo "fzf tarball checksum failed"
else
    sudo tar -xf /tmp/fzf.tar.gz -C /usr/local/bin fzf
fi
rm -f /tmp/fzf.tar.gz

# rust tools
curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
cargo binstall flamegraph -y
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# js stuffs
curl -fsSL https://bun.sh/install | bash
echo 'export BUN_INSTALL="$HOME/.bun"' >> ~/.bashrc
echo 'export PATH="$BUN_INSTALL/bin:$PATH"' >> ~/.bashrc
export BUN_INSTALL="$HOME/.bun"
export PATH="$BUN_INSTALL/bin:$PATH"
cd frontend && bun i