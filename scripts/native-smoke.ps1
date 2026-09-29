[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [ValidateRange(5, 120)]
    [int]$StartupTimeoutSeconds = 30
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$binaryPath = Join-Path $repoRoot 'target\debug\buttonscli.exe'
$smokeHome = Join-Path ([System.IO.Path]::GetTempPath()) "buttonscli-native-smoke-$([guid]::NewGuid().ToString('N'))"
$appData = Join-Path $smokeHome 'AppData'
$roaming = Join-Path $appData 'Roaming'
$local = Join-Path $appData 'Local'

New-Item -ItemType Directory -Path $roaming, $local -Force | Out-Null

Push-Location $repoRoot
try {
    if (-not $SkipBuild) {
        & cargo build --bin buttonscli
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build --bin buttonscli failed with exit code $LASTEXITCODE"
        }
    }

    if (-not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
        throw "Native executable not found: $binaryPath"
    }

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $binaryPath
    $startInfo.WorkingDirectory = $repoRoot
    $startInfo.UseShellExecute = $false
    $startInfo.Environment['USERPROFILE'] = $smokeHome
    $startInfo.Environment['HOME'] = $smokeHome
    $startInfo.Environment['APPDATA'] = $roaming
    $startInfo.Environment['LOCALAPPDATA'] = $local

    $process = [System.Diagnostics.Process]::Start($startInfo)
    $deadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
    $windowFound = $false

    while ([DateTime]::UtcNow -lt $deadline) {
        $process.Refresh()
        if ($process.HasExited) {
            throw "Native GUI exited during startup with code $($process.ExitCode)"
        }
        if ($process.MainWindowHandle -ne [IntPtr]::Zero) {
            $windowFound = $true
            break
        }
        Start-Sleep -Milliseconds 250
    }

    if (-not $windowFound) {
        throw "Native GUI did not create a main window within $StartupTimeoutSeconds seconds"
    }

    Start-Sleep -Seconds 2
    Write-Output "Native GUI opened and stayed alive (PID $($process.Id))."
    Write-Output "Isolated native data root: $smokeHome\.buttonscli-native"
}
finally {
    if ($null -ne $process -and -not $process.HasExited) {
        if (-not $process.CloseMainWindow() -or -not $process.WaitForExit(5000)) {
            $process.Kill($true)
            $process.WaitForExit()
        }
    }
    if ($null -ne $process) {
        $process.Dispose()
    }
    Pop-Location
}
