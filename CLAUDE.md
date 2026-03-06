# Blink - GPU-Accelerated Browser IDE

## Project Structure
- `crates/blink-core/` - Rust core compiled to WebAssembly (text buffer, editor logic, WebGPU renderer)
- `packages/blink-web/` - React/TypeScript frontend (UI shell, file system, wasm bridge)
- `scripts/` - Build and development scripts

## Build Commands
- `bash scripts/build-wasm.sh` - Compile Rust to WebAssembly (requires wasm-pack)
- `cd packages/blink-web && bun install && bun run dev` - Start frontend dev server
- `bash scripts/dev.sh` - Build wasm + start dev server
- `cd crates/blink-core && cargo test` - Run Rust unit tests
c
## Architecture
- **Rendering**: WebGPU via wgpu crate, rendered to a canvas element
- **Text Buffer**: Piece table implementation in Rust
- **File System**: File System Access API with IndexedDB fallback (local-first)
- **Collaboration**: Future CRDT-based system with relay server (no file storage on server)

## Key Decisions
- Local-first: files never leave the user's machine unless they explicitly enable collaboration
- React for UI chrome (tabs, sidebar, menus), Rust/WebGPU for editor canvas
- Vite for frontend bundling with wasm plugin support
