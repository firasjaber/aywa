param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("preflight", "manifest", "load-check")]
    [string]$Command,
    [string]$Argument = "parakeet-v0.2-en"
)

$cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"

if (Test-Path $cargo) {
    & $cargo run --manifest-path src-tauri/Cargo.toml --bin model_manager_probe -- $Command $Argument
} else {
    cargo run --manifest-path src-tauri/Cargo.toml --bin model_manager_probe -- $Command $Argument
}

exit $LASTEXITCODE
