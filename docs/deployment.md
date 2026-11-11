# Deployment & Operations

## Installation

### Homebrew (Recommended)

Install the CLI via the official tap:

```bash
brew install Delnegend/tap/artefact-cli
```

### Prebuilt Release Binaries

Download native Linux x86_64 binaries directly from GitHub Releases:

```bash
URL=$(curl -fsSL https://api.github.com/repos/Delnegend/artefact/releases/latest | grep -o 'https://[^"]*linux-amd64\.tar\.xz')
curl -fsSL -o artefact.tar.xz "$URL" && tar -xJf artefact.tar.xz --strip-components=1
```

## Web & WebAssembly Deployment

The web interface is a zero-install static PWA built with Vite and Vue 3. It executes the solver client-side via WebAssembly and WebGPU without uploading images to any remote server:

```bash
# Build WASM solver module
just build wasm

# Build static web assets for deployment (outputs to frontend/dist)
just build web
```

Serve `frontend/dist` with any static web host, CDN, or GitHub Pages.

## Upgrades

Keep the native CLI updated through Homebrew:

```bash
brew upgrade Delnegend/tap/artefact-cli
```
