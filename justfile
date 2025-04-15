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

_ensure_deps:
	#!/usr/bin/env bash
	if ! command -v zip &> /dev/null; then
		echo "zip is not installed"
		exit 1
	fi
	if ! command -v tar &> /dev/null; then
		echo "tar is not installed"
		exit 1
	fi

test: _ensure_deps
	#!/usr/bin/env python3
	import os, shutil, subprocess

	def run(cmd):
		print(cmd)
		subprocess.run(cmd, shell=True, check=True)

	version = None
	with open('artefact-cli/Cargo.toml') as f:
		for line in f:
			if 'version' in line:
				version = line.split('"')[1]
				break

	def build(target, arch):
		run(f'rustup target add {target}')
		run(f'cargo build --bin artefact-cli --release --target {target}')
		if "windows" in target:
			exe = 'artefact-cli.exe'
		else:
			exe = 'artefact-cli'
		if not os.path.exists(f'target/{target}/release/{exe}'):
			raise Exception(f'Build failed for {target}')

		ext = '.zip' if "windows" in target else '.tar.gz'
		pkg = 'zip -j' if ext == '.zip' else 'tar -czvf'
		ver = f'-{version}' if version is not None else ''
		run(f'{pkg} dist-cli/artefact-cli{ver}-{arch}{ext} target/{target}/release/{exe}')

	shutil.rmtree('dist-cli', ignore_errors=True)
	os.makedirs('dist-cli', exist_ok=True)

	build('i686-pc-windows-gnu', 'win-32')
	build('x86_64-pc-windows-gnu', 'win-64')
	build('i686-unknown-linux-gnu', 'linux-32')
	build('x86_64-unknown-linux-gnu', 'linux-64')