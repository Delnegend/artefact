test-base-420:
	RUSTFLAGS="-Ctarget-cpu=native" cargo run --release --bin artefact-cli -- lena-base-420.jpg -y

flame:
	CARGO_PROFILE_RELEASE_DEBUG=true RUSTFLAGS="-Ctarget-cpu=native" cargo flamegraph --bin artefact-cli --release -- lena-base-420.jpg -y

build-win-32:
	RUSTFLAGS="-Ctarget-cpu=native" cargo build --bin artefact-cli --release --target i686-pc-windows-gnu

build-win-64:
	RUSTFLAGS="-Ctarget-cpu=native" cargo build --bin artefact-cli --release --target x86_64-pc-windows-gnu

build-linux-32:
	RUSTFLAGS="-Ctarget-cpu=native" cargo build --bin artefact-cli --release --target i686-unknown-linux-gnu

build-linux-64:
	RUSTFLAGS="-Ctarget-cpu=native" cargo build --bin artefact-cli --release --target x86_64-unknown-linux-gnu

build-wasm:
	#!/usr/bin/env bash
	rm -rf src/utils/artefact-wasm
	cd artefact-wasm
	wasm-pack build --target web --out-dir ../src/utils/artefact-wasm
	cd ..
	rm -f src/utils/artefact-wasm/.gitignore

dev:
	pnpm nuxt dev

generate:
	#!/usr/bin/env bash
	pnpm nuxt generate
	cp .nuxt/dist/client/manifest.webmanifest .output/public/manifest.webmanifest

preview:
	pnpm nuxt preview

postinstall:
	pnpm nuxt prepare

lint:
	pnpm eslint --fix --cache .

tidy:
	#!/usr/bin/env bash
	cargo fmt
	cargo clippy