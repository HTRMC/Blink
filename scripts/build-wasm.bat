@echo off
setlocal

echo Building blink-core to WebAssembly...

where wasm-pack >nul 2>&1
if %errorlevel% neq 0 (
    echo wasm-pack not found. Install it with:
    echo   cargo install wasm-pack
    exit /b 1
)

cd /d "%~dp0..\crates\blink-core"
wasm-pack build --target web --out-dir "%~dp0..\packages\blink-web\wasm" --out-name blink_core

if exist "%~dp0..\packages\blink-web\wasm\.gitignore" del "%~dp0..\packages\blink-web\wasm\.gitignore"
if exist "%~dp0..\packages\blink-web\wasm\package.json" del "%~dp0..\packages\blink-web\wasm\package.json"

echo WASM build complete! Output: packages\blink-web\wasm\
