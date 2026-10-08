[CmdletBinding()]
param([switch]$SkipBuild)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$binaryPath = Join-Path $repoRoot 'target\debug\buttonscli.exe'
$migrationFlags = @('BUTTONSCLI_NATIVE_DEV_AI_HELP', 'BUTTONSCLI_NATIVE_DEV_THEME_GENERATOR', 'BUTTONSCLI_NATIVE_DEV_REMOTE_CONTROL')
$previousValues = @{}
foreach ($flag in $migrationFlags) {
    $previousValues[$flag] = [Environment]::GetEnvironmentVariable($flag, 'Process')
}
Push-Location $repoRoot
try {
    if (-not $SkipBuild) {
        & cargo build --bin buttonscli
        if ($LASTEXITCODE -ne 0) { throw 'The native debug build failed.' }
    }
    if (-not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) { throw 'Build the native app first.' }
    foreach ($flag in $migrationFlags) { [Environment]::SetEnvironmentVariable($flag, '1', 'Process') }
    Write-Output 'Launching the current debug build with AI Help, AI themes and CLI/MCP development access.'
    & $binaryPath
} finally {
    foreach ($flag in $migrationFlags) { [Environment]::SetEnvironmentVariable($flag, $previousValues[$flag], 'Process') }
    Pop-Location
}
