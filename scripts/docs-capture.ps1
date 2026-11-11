<#
.SYNOPSIS
Re-captures the docs screenshots in docs/guide/images/.

.DESCRIPTION
Builds the library tests, then runs the ignored native_docs_capture probe:
an owned window with simulated panes (no shells, no network, no personal
profile) goes through the shot list in src/app/docs_capture.rs. Each shot is
cropped to its anchors and written only when the picture really changed.
See docs/dev/DOCS-SCREENSHOTS.md.

.EXAMPLE
./scripts/docs-capture.ps1
.EXAMPLE
./scripts/docs-capture.ps1 -Only hero,colors-row
.EXAMPLE
./scripts/docs-capture.ps1 -DryRun     # writes to target/docs-capture/preview
.EXAMPLE
./scripts/docs-capture.ps1 -Locale de  # writes to docs/guide/images/de
#>
[CmdletBinding()]
param(
    # Shot names (file or file stem), comma separated. Default: every shot.
    [string[]]$Only,
    # App language; non-English shots go to docs/guide/images/<locale>/.
    [string]$Locale = 'en',
    # Write somewhere else instead of the docs folder.
    [string]$Out,
    # Preview: write to target/docs-capture/preview, leave the docs alone.
    [switch]$DryRun,
    # Rewrite files even when the picture did not change.
    [switch]$Force
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$work = Join-Path $repoRoot 'target\docs-capture'
if ($DryRun -and -not $Out) { $Out = Join-Path $work 'preview' }

Push-Location $repoRoot
try {
    $buildArgs = @('test', '--locked', '--lib', '--no-run', '--message-format=json')
    Write-Host 'Building the test binary…'
    $exe = cargo @buildArgs 2>$null |
        ForEach-Object { try { $_ | ConvertFrom-Json } catch { $null } } |
        Where-Object { $_.reason -eq 'compiler-artifact' -and $_.target.name -eq 'buttonscli' -and $_.profile.test -and $_.executable } |
        Select-Object -Last 1 -ExpandProperty executable
    if (-not $exe) { throw 'cargo did not report the library test binary; run `cargo test --locked --lib --no-run` to see the error.' }

    # A scratch home on top of the probe's own temporary profile, so nothing
    # can read or write the real ~/.buttonscli-native.
    $scratch = Join-Path ([System.IO.Path]::GetTempPath()) "buttonscli-docs-capture-home-$([guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path (Join-Path $scratch 'AppData\Roaming'), (Join-Path $scratch 'AppData\Local') -Force | Out-Null
    $saved = @{}
    $environment = @{
        USERPROFILE = $scratch
        HOME = $scratch
        APPDATA = (Join-Path $scratch 'AppData\Roaming')
        LOCALAPPDATA = (Join-Path $scratch 'AppData\Local')
        BUTTONSCLI_DOCS_CAPTURE_LOCALE = $Locale
        BUTTONSCLI_DOCS_CAPTURE_WORK = $work
        BUTTONSCLI_DOCS_CAPTURE_ONLY = ($Only -join ',')
        BUTTONSCLI_DOCS_CAPTURE_OUT = $Out
        BUTTONSCLI_DOCS_CAPTURE_FORCE = $(if ($Force) { '1' } else { $null })
    }
    foreach ($name in $environment.Keys) {
        $saved[$name] = [Environment]::GetEnvironmentVariable($name)
        # PowerShell turns $null into "" for .NET strings, so unset explicitly.
        if ($environment[$name]) {
            Set-Item -LiteralPath "Env:$name" -Value $environment[$name]
        } else {
            Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
        }
    }
    try {
        Write-Host 'Capturing (a window opens; leave it alone until it closes)…'
        & $exe 'app::docs_capture::probe::native_docs_capture' --exact --ignored --test-threads=1 --nocapture |
            Where-Object { $_ -match 'docs capture:' } |
            # The first line follows the test runner's "test … ..." prefix.
            ForEach-Object { Write-Host ($_ -replace '^.*?(docs capture:)', '$1') }
        $exitCode = $LASTEXITCODE
    } finally {
        foreach ($name in $saved.Keys) {
            if ($null -ne $saved[$name]) {
                Set-Item -LiteralPath "Env:$name" -Value $saved[$name]
            } else {
                Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
            }
        }
        Remove-Item -LiteralPath $scratch -Recurse -Force -ErrorAction SilentlyContinue
    }

    $reportPath = Join-Path $work 'report.json'
    if (Test-Path $reportPath) {
        $report = Get-Content $reportPath -Raw | ConvertFrom-Json
        Write-Host ''
        $report | Format-Table file, status, width, height, @{ Name = 'changed %'; Expression = { $_.changed_percent } }, error -AutoSize | Out-Host
        $counts = $report | Group-Object status | ForEach-Object { "$($_.Count) $($_.Name)" }
        Write-Host ("Summary: " + ($counts -join ', '))
        Write-Host "Full-window captures for checking crops: $work"
    }
    if (-not $Out) {
        Write-Host ''
        git status --short -- docs/guide/images
    }
    if ($exitCode -ne 0) { throw "native_docs_capture failed (exit $exitCode)." }
} finally {
    Pop-Location
}
