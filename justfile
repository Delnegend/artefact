@default:
	just --choose

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
	rm -rf frontend/src/utils/artefact-wasm
	cd backend/artefact-wasm
	wasm-pack build --target web --out-dir ../../frontend/src/utils/artefact-wasm
	cd ..
	rm -f frontend/src/utils/artefact-wasm/.gitignore

dev:
	cd frontend && bun x nuxt dev  --no-fork

build-web: build-wasm
	#!/usr/bin/env bash
	cd frontend
	bun x nuxt generate
	cp node_modules/.cache/nuxt/.nuxt/dist/client/manifest.webmanifest .output/public/manifest.webmanifest

lint:
	#!/usr/bin/env bash
	cd frontend && \
		bun x oxlint --import-plugin -D correctness -D perf \
		--ignore-pattern src/dev-dist/**/*.* \
		--ignore-pattern src/utils/artefact-wasm/**/*.* && \
		bun x prettier -l -w "**/*.{js,ts,vue,json,css}"

lint-rust:
	#!/usr/bin/env bash
	cargo fmt
	cargo clippy

# Build release binaries for different platforms to release on GitHub
build-cross-platform:
	#!/usr/bin/env python3
	import os, shutil, subprocess

	if not shutil.which('tar') or not shutil.which('zip'):
		raise Exception("tar and zip must be installed and in PATH")

	def run(cmd):
		print(cmd)
		subprocess.run(cmd, shell=True, check=True)

	version = None
	with open('backend/artefact-cli/Cargo.toml') as f:
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