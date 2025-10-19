#!/usr/bin/env bash

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly
echo 'export CARGO_HOME="/usr/local/cargo"' >> ~/.bashrc
echo 'export PATH="/usr/local/cargo/bin:$PATH"' >> ~/.bashrc

# mold
ver="2.40.4"
curl -L -o /tmp/mold.tar.gz https://github.com/rui314/mold/releases/download/v$ver/mold-$ver-x86_64-linux.tar.gz
checksum=$(openssl dgst -sha3-512 /tmp/mold.tar.gz | awk '{print $2}')
expected="1faf14077e25608993985b2e1e307ce2f36dc5653baedd6b4460775b1058d4120f7e225206b96cee861e0267b06aeb9dfed010e8dccedaacfb6f2672df7b7490"
if [ ! "$checksum" = "$expected" ]; then
    echo "mold tarball checksum failed\nexpected: $expected\ngot: $checksum"
else
    sudo rm -rf /usr/local/cargo/mold*
    sudo tar -xf /tmp/mold.tar.gz -C /usr/local/cargo mold-$ver-x86_64-linux
fi
rm -f /tmp/mold.tar.gz

# configure cargo to use mold
rm -f /usr/local/cargo/config.toml
printf "[target.x86_64-unknown-linux-gnu]\nlinker = \"clang\"\nrustflags = [\"-C\", \"link-arg=-fuse-ld=/usr/local/cargo/mold-$ver-x86_64-linux/bin/mold\"]" > /usr/local/cargo/config.toml

# just
ver="1.43.0"
curl -L -o /tmp/just.tar.gz https://github.com/casey/just/releases/download/$ver/just-$ver-x86_64-unknown-linux-musl.tar.gz
checksum=$(openssl dgst -sha3-512 /tmp/just.tar.gz | awk '{print $2}')
expected="84d1ce2248381502507a5b6a67b3b6bc80fd98e52f8604f2628a087405272b10b371d235b11051f49176aa9fb4c17f6c7cb1d96e05f755a799b2ef05b6d3e3e8"
if [ ! "$checksum" = "$expected" ]; then
    echo "just tarball checksum failed\nexpected: $expected\ngot: $checksum"
else
    sudo rm -rf /usr/local/bin/just
    sudo tar -xf /tmp/just.tar.gz -C /usr/local/bin just
fi
rm -f /tmp/just.tar.gz
echo 'alias j=just' >> ~/.bashrc
echo 'eval "$(just --completions bash)"' >> ~/.bashrc
echo 'complete -F _just j' >> ~/.bashrc

# fzf
ver="0.66.0"
curl -L -o /tmp/fzf.tar.gz https://github.com/junegunn/fzf/releases/download/v$ver/fzf-$ver-linux_amd64.tar.gz
checksum=$(openssl dgst -sha3-512 /tmp/fzf.tar.gz | awk '{print $2}')
expected="1710205b6f924c78ebfc6b43e1697e4cf4ba168d7970196f23effb4f125e956a76a07ae8a26dfcd1a4a5b26435b2670bb840b7d1c4ea92befef09789d17068b0"
if [ ! "$checksum" = "$expected" ]; then
    echo "fzf tarball checksum failed\nexpected: $expected\ngot: $checksum"
else
    sudo rm -rf /usr/local/bin/fzf
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