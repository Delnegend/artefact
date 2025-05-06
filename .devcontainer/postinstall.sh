#!/bin/zsh

sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly
echo 'export CARGO_HOME="/usr/local/cargo"' >> ~/.zshrc
echo 'export PATH="/usr/local/cargo/bin:$PATH"' >> ~/.zshrc

# mold
ver="2.39.0"
curl -L -o /tmp/mold.tar.gz https://github.com/rui314/mold/releases/download/v$ver/mold-$ver-x86_64-linux.tar.gz
checksum=$(openssl dgst -sha3-512 /tmp/mold.tar.gz | awk '{print $2}')
expected="8c650f3eee61eddaba42ea2783c4c6ebb5fe30976be83113cdd0fcfa0ae1a107b1e6c7ed94a187c920d2a2e1c6bae045795e70da10764710523d98a6ff72ebeb"
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
ver="1.40.0"
curl -L -o /tmp/just.tar.gz https://github.com/casey/just/releases/download/$ver/just-$ver-x86_64-unknown-linux-musl.tar.gz
checksum=$(openssl dgst -sha3-512 /tmp/just.tar.gz | awk '{print $2}')
expected="27f317b6ca704395dbad34c078f57d140d6d3e1f147a29b5c7563489885525a203bb289b02ec05e52282d25dfe446a60e11554a58addc682ad17f94b6b100cb9"
if [ ! "$checksum" = "$expected" ]; then
    echo "just tarball checksum failed\nexpected: $expected\ngot: $checksum"
else
    sudo rm -rf /usr/local/bin/just
    sudo tar -xf /tmp/just.tar.gz -C /usr/local/bin just
fi
rm -f /tmp/just.tar.gz
# add j=just alias
echo 'alias j=just' >> ~/.zshrc

# fzf
ver="0.61.3"
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

curl -fsSL https://bun.sh/install | bash
echo 'export BUN_INSTALL="$HOME/.bun"' >> ~/.zshrc
echo 'export PATH="$BUN_INSTALL/bin:$PATH"' >> ~/.zshrc
bun i