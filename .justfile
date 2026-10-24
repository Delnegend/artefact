@default:
	just --choose

dev:
	cd frontend && bun x vite

# check code for: rust (backend), js (frontend)
# formatting is applied first (rustfmt + oxfmt), then linted/type-checked
# default: all — rust checks ensure all pipelines always buildable (no silent regression)
check kind="all":
	#!/usr/bin/env bash
	set -euo pipefail

	if [[ "{{kind}}" = "all" || "{{kind}}" = "js" ]]; then
		cd frontend
		bun x oxlint --import-plugin -D correctness -D perf \
			--ignore-pattern dev-dist/**/*.* \
			--ignore-pattern src/utils/artefact-wasm/**/*.*
		bun x oxfmt
		# vue-tsc cannot run under bun (its Volar plugin silently fails to load,
		# producing bogus TS2307 "Cannot find module './X.vue'" errors) — so fail
		# fast with the real cause instead of 32 misleading type errors. Never
		# shim this with a node -> bun symlink: a lying runtime is worse than a
		# missing one.
		command -v node >/dev/null || { echo "error: just check js needs node on PATH (vue-tsc cannot run under bun)" >&2; exit 1; }
		bun x vue-tsc --build
		cd -
	fi

	if [[ "{{kind}}" = "all" || "{{kind}}" = "rust" ]]; then
		cargo fmt
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
		# GPU smoke/equivalence tests really run on CI's lavapipe; plain
		# `cargo test` elsewhere may still skip when no adapter is present.
		ARTEFACT_REQUIRE_GPU=1 cargo test --workspace --all-features
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
		bun x vite build
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

# Language-agnostic version contract for Delnegend/actions/resolve-release.
# Single truth: Cargo.toml workspace version; bump syncs frontend/package.json.
version action ver="":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{action}}" in
      get)
        python3 -c "import tomllib;d=tomllib.load(open('Cargo.toml','rb'));print(d.get('workspace',{}).get('package',{}).get('version') or d.get('package',{}).get('version',''))"
        ;;
      set|bump)
        [ -n "{{ver}}" ] || { echo "usage: just version set X.Y.Z" >&2; exit 1; }
        VERSION="{{ver}}"
        sed -i -E "0,/^version = \".*\"/s//version = \"${VERSION}\"/" Cargo.toml
        cargo check -p artefact-cli --quiet
        cd frontend && npm version --no-git-tag-version "${VERSION}"
        ;;
      *) echo "usage: just version get|set X.Y.Z" >&2; exit 1 ;;
    esac
