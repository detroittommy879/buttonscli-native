param([switch]$Serve, [int]$Port = 8080)
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    if (Get-Command wasm-pack -ErrorAction SilentlyContinue) {
        wasm-pack build --target web --out-dir web/pkg --no-default-features -- --locked
    } else {
        npx --yes wasm-pack@0.15.0 build --target web --out-dir web/pkg --no-default-features -- --locked
    }
    if ($LASTEXITCODE -ne 0) { throw 'Browser build failed.' }
    if ($Serve) {
        Write-Host "Preview: http://127.0.0.1:$Port"
        python -m http.server $Port --bind 127.0.0.1 --directory web
    }
} finally { Pop-Location }
