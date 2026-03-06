# Blink

A GPU-accelerated, local-first browser IDE built with Rust, WebAssembly, and React.

- **WebGPU rendering** via the Rust `wgpu` crate — the editor canvas is rendered on the GPU, not the DOM
- **Local-first** — files stay on your machine using the File System Access API (IndexedDB fallback)
- **Collaboration-ready** — future CRDT-based real-time editing with a lightweight relay server

## Prerequisites

- [Rust](https://rustup.rs) (on Windows, download and run `rustup-init.exe`)
- [Bun](https://bun.sh) v1.0+
- [wasm-pack](https://rustwasm.github.io/wasm-pack/)

## Setup

```bash
# 1. Install wasm-pack (after installing Rust)
cargo install wasm-pack

# 2. Install frontend dependencies
cd packages/blink-web
bun install
cd ../..
```

## Development

### Using Git Bash (recommended on Windows)

```bash
bash scripts/build-wasm.sh
cd packages/blink-web
bun run dev
```

### Using CMD / PowerShell

```
scripts\build-wasm.bat
cd packages\blink-web
bun run dev
```

### All-in-one

```bash
# Git Bash
bash scripts/dev.sh

# CMD / PowerShell
scripts\dev.bat
```

The dev server starts at **http://localhost:3000**.

> WebGPU requires Chrome 113+ or Edge 113+.

## Project Structure

```
Blink/
├── crates/blink-core/       # Rust core → WebAssembly
│   └── src/
│       ├── buffer.rs         # Piece table text buffer
│       ├── editor.rs         # Editor state (cursor, content)
│       ├── renderer.rs       # WebGPU renderer (wgpu)
│       └── shader.wgsl       # WGSL shaders
├── packages/blink-web/       # React frontend
│   └── src/
│       ├── components/       # EditorCanvas, Sidebar, TabBar, StatusBar
│       └── hooks/            # File System Access API + IndexedDB
└── scripts/                  # Build and dev scripts (.sh + .bat)
```

## Architecture

| Layer | Tech | Responsibility |
|-------|------|----------------|
| Rendering | Rust + wgpu → WebAssembly | GPU-accelerated editor canvas |
| Text Buffer | Rust (piece table) | Efficient insert/delete operations |
| UI Shell | React + TypeScript | Tabs, file explorer, menus, status bar |
| File I/O | File System Access API | Local file read/write without a server |
| Storage Fallback | IndexedDB | For browsers without File System Access |

## License

MIT
