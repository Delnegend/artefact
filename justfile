@default:
	just --choose

dev:
	cd frontend && bun x nuxt dev  --no-fork

# check code for: rust (backend), js (frontend)
# default: all — rust checks ensure all pipelines always buildable (no silent regression)
check kind="all":
	#!/usr/bin/env bash
	set -euo pipefail

	if [[ "{{kind}}" = "all" || "{{kind}}" = "js" ]]; then
		cd frontend
		bun x oxlint --import-plugin -D correctness -D perf \
			--ignore-pattern src/dev-dist/**/*.* \
			--ignore-pattern src/utils/artefact-wasm/**/*.*
		bun x prettier -l -w "**/*.{js,ts,vue,json,css}"
		cd -
	fi

	if [[ "{{kind}}" = "all" || "{{kind}}" = "rust" ]]; then
		cargo fmt -- --check
		# always-buildable: scalar / simd are both compiled
		cargo check --workspace
		cargo check --workspace --all-features
		# native scalar dispatch (workspace feature-unification otherwise selects simd)
		cargo check -p artefact-core
		# wasm — std::simd must build for wasm32
		cargo check -p artefact-core --target wasm32-unknown-unknown
		cargo check -p artefact-core --target wasm32-unknown-unknown --features simd
		# wasm — wgpu's WebGPU backend must build too
		cargo check -p artefact-core --target wasm32-unknown-unknown --features simd,gpu
		cargo clippy --workspace --all-features
		# decode/fixture regression tests (zune-jpeg + pipelines + verify)
		cargo test --workspace --all-features
	fi

# build: native CLI, wasm, or web
build target="native":
	#!/usr/bin/env bash

	if [[ "{{target}}" = "wasm" ]]; then
		rm -rf frontend/src/utils/artefact-wasm
		(cd backend/artefact-wasm && wasm-pack build --target web --out-dir ../../frontend/src/utils/artefact-wasm)
		rm -f frontend/src/utils/artefact-wasm/.gitignore
		exit 0
	fi

	if [[ "{{target}}" = "web" ]]; then
		cd frontend
		bun x nuxt generate
		cp node_modules/.cache/nuxt/.nuxt/dist/client/manifest.webmanifest .output/public/manifest.webmanifest
		exit 0
	fi

	if [[ "{{target}}" != "native" ]]; then
		echo "Unknown target: {{target}} (expected: native, wasm, web)"
		exit 1
	fi

	echo "Building native CLI (release)"
	cargo build --bin artefact-cli --release

flame chroma="420":
	#!/usr/bin/env bash

	CHROMA="{{chroma}}"

	valid_chromas=("j444" "j422" "j420" "420" "422" "444")
	if [[ ! " ${valid_chromas[@]} " =~ "$CHROMA" ]]; then
		echo "Invalid chroma subsampling: ${CHROMA}"
		echo "Valid options are: ${valid_chromas[*]}"
		exit 1
	fi

	CARGO_PROFILE_RELEASE_DEBUG=true RUSTFLAGS="-Ctarget-cpu=native" cargo flamegraph --bin artefact-cli --release -- assets/sample.${CHROMA}.input.jpg -y

# generate synthetic sample.png via ffmpeg (see scripts/generate-sample.sh)
generate-sample output="assets/sample.png":
	./scripts/generate-sample.sh "{{output}}"

alias sample := generate-sample