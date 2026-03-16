@echo off
:: Refresh PATH from registry to pick up new installs
for /f "tokens=2*" %%a in ('reg query "HKCU\Environment" /v Path 2^>nul') do set "USER_PATH=%%b"
for /f "tokens=2*" %%a in ('reg query "HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment" /v Path 2^>nul') do set "SYS_PATH=%%b"
set "PATH=%SYS_PATH%;%USER_PATH%"

cd /d "%~dp0receiver"
cargo build --release --quiet && .\target\release\screen-receiver.exe %*
