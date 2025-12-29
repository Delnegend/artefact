@default:
	just --choose

test-base-420:
	RUSTFLAGS="-Ctarget-cpu=native" cargo run --release --bin artefact-cli -- lena-base-420.jpg -y

flame:
	CARGO_PROFILE_RELEASE_DEBUG=true RUSTFLAGS="-Ctarget-cpu=native" cargo flamegraph --bin artefact-cli --release -- lena-base-420.jpg -y

dev:
	cd frontend && bun x nuxt dev  --no-fork

lint kind="all":
	#!/usr/bin/env bash

	if [[ "{{kind}}" = "all" || "{{kind}}" = "js" ]]; then
		cd frontend
		bun x oxlint --import-plugin -D correctness -D perf \
			--ignore-pattern src/dev-dist/**/*.* \
			--ignore-pattern src/utils/artefact-wasm/**/*.*
		bun x prettier -l -w "**/*.{js,ts,vue,json,css}"
		cd -
	fi

	if [[ "{{kind}}" = "all" || "{{kind}}" = "rust" ]]; then
		cargo fmt
		cargo clippy
	fi

install-deps target="all" arch="":
	#!/usr/bin/env bash

	if [[ "{{target}}" = "all" ]]; then
		for t in win linux; do
			for a in 32 64; do
				just install-deps $t $a
			done
		done
		exit 0
	fi

	if [[ "{{target}}" = "wasm" || "{{target}}" = "web" ]]; then
		echo "No dependencies needed for target: {{target}}"
		exit 0
	fi

	if [[ "{{target}}" != "win" && "{{target}}" != "linux" ]]; then
		echo "Unsupported target: {{target}}"
		exit 1
	fi
	if [[ "{{arch}}" != "32" && "{{arch}}" != "64" ]]; then
		echo "Unsupported architecture: {{arch}}"
		exit 1
	fi

	if [[ "{{target}}" = "win" && "{{arch}}" = "32" ]]; then
		rustup target add i686-pc-windows-gnu
		sudo apt-get install gcc-mingw-w64-i686 -y
	elif [[ "{{target}}" = "win" && "{{arch}}" = "64" ]]; then
		rustup target add x86_64-pc-windows-gnu
		sudo apt-get install gcc-mingw-w64-x86-64 -y
	elif [[ "{{target}}" = "linux" && "{{arch}}" = "32" ]]; then
		rustup target add i686-unknown-linux-musl
	elif [[ "{{target}}" = "linux" && "{{arch}}" = "64" ]]; then
		rustup target add x86_64-unknown-linux-musl
	fi

build target="linux" arch="64":
	#!/usr/bin/env bash

	if [[ "{{target}}" = "wasm" ]]; then
		rm -rf frontend/src/utils/artefact-wasm
		cd backend/artefact-wasm
		wasm-pack build --target web --out-dir ../../frontend/src/utils/artefact-wasm
		cd ..
		rm -f frontend/src/utils/artefact-wasm/.gitignore
		exit 0
	fi

	if [[ "{{target}}" = "web" ]]; then
		cd frontend
		bun x nuxt generate
		cp node_modules/.cache/nuxt/.nuxt/dist/client/manifest.webmanifest .output/public/manifest.webmanifest
		exit 0
	fi

	if [[ "{{target}}" != "win" && "{{target}}" != "linux" ]]; then
		echo "Unsupported target: {{target}}"
		exit 1
	fi
	if [[ "{{arch}}" != "32" && "{{arch}}" != "64" ]]; then
		echo "Unsupported architecture: {{arch}}"
		exit 1
	fi

	export RUSTFLAGS="-Ctarget-cpu=native"
	if [[ "{{target}}" = "win" && "{{arch}}" = "32" ]]; then
		echo "Building for Windows 32-bit"
		cargo build --bin artefact-cli --release --target i686-pc-windows-gnu
	elif [[ "{{target}}" = "win" && "{{arch}}" = "64" ]]; then
		echo "Building for Windows 64-bit"
		cargo build --bin artefact-cli --release --target x86_64-pc-windows-gnu
	elif [[ "{{target}}" = "linux" && "{{arch}}" = "32" ]]; then
		echo "Building for Linux 32-bit"
		cargo build --bin artefact-cli --release --target i686-unknown-linux-musl
	elif [[ "{{target}}" = "linux" && "{{arch}}" = "64" ]]; then
		echo "Building for Linux 64-bit"
		cargo build --bin artefact-cli --release --target x86_64-unknown-linux-musl
	fi

release target="linux" arch="64":
	#!/usr/bin/env bash

	if [[ "{{target}}" = "all" ]]; then
		for t in win linux; do
			for a in 32 64; do
				just release $t $a
			done
		done
		exit 0
	fi

	if [[ "{{target}}" != "win" && "{{target}}" != "linux" ]]; then
		echo "Unsupported target: {{target}}"
		exit 1
	fi
	if [[ "{{arch}}" != "32" && "{{arch}}" != "64" ]]; then
		echo "Unsupported architecture: {{arch}}"
		exit 1
	fi

	just build {{target}} {{arch}}
	version=$(grep '^version =' backend/artefact-cli/Cargo.toml | head -n1 | cut -d'"' -f2)
	if [[ -z "$version" ]]; then
		echo "Could not determine version from Cargo.toml"
		exit 1
	fi
	mkdir -p releases

	if [[ "{{target}}" = "win" && "{{arch}}" = "32" ]]; then
		zip -j releases/artefact-cli-${version}-win-32.zip target/i686-pc-windows-gnu/release/artefact-cli.exe
	elif [[ "{{target}}" = "win" && "{{arch}}" = "64" ]]; then
		zip -j releases/artefact-cli-${version}-win-64.zip target/x86_64-pc-windows-gnu/release/artefact-cli.exe
	elif [[ "{{target}}" = "linux" && "{{arch}}" = "32" ]]; then
		tar -czvf releases/artefact-cli-${version}-linux-32.tar.gz -C target/i686-unknown-linux-musl/release artefact-cli
	elif [[ "{{target}}" = "linux" && "{{arch}}" = "64" ]]; then
		tar -czvf releases/artefact-cli-${version}-linux-64.tar.gz -C target/x86_64-unknown-linux-musl/release artefact-cli
	fi
