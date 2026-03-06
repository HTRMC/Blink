@echo off
setlocal

call "%~dp0build-wasm.bat"
if %errorlevel% neq 0 exit /b 1

cd /d "%~dp0..\packages\blink-web"
bun run dev
