@default:
	just --choose

dev:
	cd frontend && bun x nuxt dev  --no-fork

# check code for: rust (backend), js (frontend)
# default: all
check kind="all":
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

# build: native CLI, wasm, or web
build target="native":
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

	if [[ "{{target}}" != "native" ]]; then
		echo "Unknown target: {{target}} (expected: native, wasm, web)"
		exit 1
	fi

	echo "Building native CLI (release)"
	cargo build --bin artefact-cli --release

# update dependencies for: rust/js
# default: all
update where="all":
	#!/usr/bin/env bash

	if [[ "{{where}}" = "all" || "{{where}}" = "js" ]]; then
		cd frontend
		bun update
		cd -
	fi

	if [[ "{{where}}" = "all" || "{{where}}" = "rust" ]]; then
		cargo update
	fi

alias encode := encode-sample-image

# generate sample images with different chroma subsampling from a base image
encode-sample-image input="assets/sample.png":
	#!/usr/bin/env bash

	INPUT="{{input}}"

	declare -A MODES=(
		["j444"]="yuvj444p"
		["j422"]="yuvj422p"
		["j420"]="yuvj420p"
		["420"]="yuv420p"
		["422"]="yuv422p"
		["444"]="yuv444p"
	)

	# Loop through each mode and convert
	for suffix in "${!MODES[@]}"; do
		pix_fmt=${MODES[$suffix]}
		filename="${input%.*}"
		output="assets/sample.${suffix}.input.jpg"

		echo "Converting $INPUT to $output using $pix_fmt..."

		ffmpeg -i "$INPUT" -pix_fmt "$pix_fmt" "$output" -y
	done

alias decode := decode-sample-image


decode-sample-image chroma="420":
	#!/usr/bin/env bash

	CHROMA="{{chroma}}"

	valid_chromas=("j444" "j422" "j420" "420" "422" "444")
	if [[ ! " ${valid_chromas[@]} " =~ "$CHROMA" ]]; then
		echo "Invalid chroma subsampling: ${CHROMA}"
		echo "Valid options are: ${valid_chromas[*]}"
		exit 1
	fi

	INPUT="assets/sample.${CHROMA}.input.jpg"
	OUTPUT="assets/sample.${CHROMA}.decoded.png"

	echo "Decoding $INPUT to $OUTPUT..."

	cargo run --bin artefact-cli -- "$INPUT" -o "$OUTPUT" -y

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