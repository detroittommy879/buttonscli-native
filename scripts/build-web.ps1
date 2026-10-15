param([switch]$Serve, [switch]$Offline, [ValidateRange(1,65535)][int]$Port = 8080)
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    if (Get-Command wasm-pack -ErrorAction SilentlyContinue) {
        wasm-pack build --target web --out-dir web/pkg --no-default-features -- --locked
    } else {
        npx --yes wasm-pack@0.15.0 build --target web --out-dir web/pkg --no-default-features -- --locked
    }
    if ($LASTEXITCODE -ne 0) { throw 'Browser build failed.' }
    $revision = (git rev-parse HEAD).Trim()
    $receipt = @{ revision = $revision; builtAt = [DateTime]::UtcNow.ToString('o'); wasmSha256 = (Get-FileHash web/pkg/buttonscli_bg.wasm -Algorithm SHA256).Hash.ToLowerInvariant() }
    $receipt | ConvertTo-Json | Set-Content web/build-info.json -Encoding utf8
    if ($Serve) {
        Write-Host "Preview: http://127.0.0.1:$Port"
        if ($Offline) {
            python -m http.server $Port --bind 127.0.0.1 --directory web
        } else {
            cargo build --locked --bin buttonscli-web
            if ($LASTEXITCODE -ne 0) { throw 'Preview server build failed.' }
            & ./target/debug/buttonscli-web --port $Port --web-root web
        }
    }
} finally { Pop-Location }
