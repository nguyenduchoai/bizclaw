@echo off
REM build.bat - Windows build script for BizClaw
cargo build --release --target x86_64-pc-windows-msvc
copy target\x86_64-pc-windows-msvc\release\bizclaw.exe packaging\windows\
copy target\x86_64-pc-windows-msvc\release\bizclaw-platform.exe packaging\windows\
makensis packaging\windows\bizclaw.nsi
