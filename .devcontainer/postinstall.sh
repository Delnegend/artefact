MOLD_VERSION=2.36.0
MOLD_MD5=0cbdd068a70ef28cad32c4005fd9f1df

sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal -c clippy rustfmt

echo 'export CARGO_HOME="/usr/local/cargo"' >> ~/.zshrc
echo 'export PATH="/usr/local/cargo/bin:$PATH"' >> ~/.zshrc

if [ ! -d /usr/local/cargo/mold-$MOLD_VERSION-x86_64-linux ]; then
    cd /usr/local/cargo
    curl -L -o mold-$MOLD_VERSION-x86_64-linux.tar.gz https://github.com/rui314/mold/releases/download/v$MOLD_VERSION/mold-$MOLD_VERSION-x86_64-linux.tar.gz
    if [ "$(md5sum mold-$MOLD_VERSION-x86_64-linux.tar.gz | awk '{print $1}')" = "$MOLD_MD5" ]; then
        tar -xvf mold-$MOLD_VERSION-x86_64-linux.tar.gz
        rm -f mold-$MOLD_VERSION-x86_64-linux.tar.gz
    else
        echo "mold-$MOLD_VERSION-x86_64-linux.tar.gz has been modified"
    fi
else
    echo "already downloaded mold-$MOLD_VERSION-x86_64-linux"
fi

# configure cargo to use mold
rm -f /usr/local/cargo/config.toml
printf "[target.x86_64-unknown-linux-gnu]\nlinker = \"clang\"\nrustflags = [\"-C\", \"link-arg=-fuse-ld=/usr/local/cargo/mold-$MOLD_VERSION-x86_64-linux/bin/mold\"]" > /usr/local/cargo/config.toml

# use binstall so we don't have to compile
curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash


# 3rd party rust-powered tools
cargo binstall flamegraph nrr -y
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Install and setup volta
curl https://get.volta.sh | bash

# Setup volta in current shell
export VOLTA_HOME="$HOME/.volta"
export PATH="$VOLTA_HOME/bin:$PATH"

# Install node and pnpm
volta install node@lts pnpm

# change pnpm store
pnpm config set store-dir ~/.pnpm-store

pnpm i
