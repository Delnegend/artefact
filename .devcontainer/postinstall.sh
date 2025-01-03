MOLD_VERSION=2.35.1
MOLD_MD5=aa65b2f71b5944d1ea8d19498e3bb750

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

cargo install flamegraph
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Install and setup volta
curl https://get.volta.sh | bash

# Setup volta in current shell
export VOLTA_HOME="$HOME/.volta"
export PATH="$VOLTA_HOME/bin:$PATH"

# Install node and pnpm
volta install node@lts
volta install pnpm

# change pnpm store
pnpm config set store-dir ~/.pnpm-store

pnpm i

