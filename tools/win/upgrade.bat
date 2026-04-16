@echo off
setlocal enabledelayedexpansion
set "SCRIPT_DIR=%~dp0"
set "RANDOMPRIME_DIR=%SCRIPT_DIR%/../.."

set "DIRECTORIES="
for /r %%d in (Cargo.toml) do (
    if exist "%%~dpdCargo.toml" (
        set "DIR=%%~dpd"
        set "DIRECTORIES=!DIRECTORIES! !DIR!"
    )
)

cd "%RANDOMPRIME_DIR%"
rustup update

for %%d in (%DIRECTORIES%) do (
    echo %%d
    cd "%%d"

    @REM cargo fix --edition --allow-dirty --allow-staged
    @REM cargo update --verbose
    cargo update
    cargo fmt
    cargo clippy --allow-dirty --allow-staged --fix
    cargo fmt
)
