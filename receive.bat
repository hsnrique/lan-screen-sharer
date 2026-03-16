@echo off
cd /d "%~dp0receiver"
cargo build --release --quiet && .\target\release\screen-receiver.exe %*
