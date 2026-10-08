[CmdletBinding()]
param([string]$Shell = 'wsl.exe -d Ubuntu-26.04 --exec htop', [switch]$CurrentTheme,
      [string]$CapturePath = (Join-Path $PSScriptRoot '..\target\terminal-tui-smoke.png'))
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$testHome = Join-Path ([System.IO.Path]::GetTempPath()) "buttonscli-tui-$([guid]::NewGuid().ToString('N'))"
$profileDir = Join-Path $testHome '.buttonscli-native\profiles\default'
New-Item -ItemType Directory -Path $profileDir, (Join-Path $testHome 'AppData\Roaming'), (Join-Path $testHome 'AppData\Local') -Force | Out-Null
$preferences = @{ localization = @{ mode='manual'; manual_locale='en'; first_run_language_confirmed=$true } }
if ($CurrentTheme) {
    $nativeRoot = Join-Path $env:USERPROFILE '.buttonscli-native'
    $active = (Get-Content -LiteralPath (Join-Path $nativeRoot 'active-profile.json') -Raw | ConvertFrom-Json).name
    $saved = (Get-Content -LiteralPath (Join-Path $nativeRoot "profiles\$active\native.json") -Raw | ConvertFrom-Json).preferences
    $preferences.typography = $saved.typography
    $themeName = ($saved.terminal_theme_id -split ':')[-1]
    $sourceTheme = Join-Path $nativeRoot "profiles\$active\themes\$themeName.json"
    New-Item -ItemType Directory -Path (Join-Path $profileDir 'themes') -Force | Out-Null
    Copy-Item -LiteralPath $sourceTheme -Destination (Join-Path $profileDir 'themes\fixture.json')
    foreach ($key in @('theme_id','terminal_theme_id','app_theme_id','effects_theme_id','gradient_theme_id')) { $preferences[$key] = 'personal:default:fixture' }
}
@{schema_version=1; revision=1; preferences=$preferences} | ConvertTo-Json -Depth 20 |
    Set-Content -LiteralPath (Join-Path $profileDir 'native.json') -Encoding utf8NoBOM
$start = [System.Diagnostics.ProcessStartInfo]::new()
$start.FileName = Join-Path $repoRoot 'target\release\buttonscli.exe'
$start.WorkingDirectory = $repoRoot
$start.UseShellExecute = $false
$start.ArgumentList.Add('--shell'); $start.ArgumentList.Add($Shell)
$start.Environment['HOME']=$testHome; $start.Environment['USERPROFILE']=$testHome
$start.Environment['APPDATA']=Join-Path $testHome 'AppData\Roaming'
$start.Environment['LOCALAPPDATA']=Join-Path $testHome 'AppData\Local'
$start.Environment['BUTTONSCLI_NATIVE_DISABLE_ACCOUNT']='1'
$start.Environment['BUTTONSCLI_NATIVE_DISABLE_REMOTE_CONFIG']='1'
$process = [System.Diagnostics.Process]::Start($start)
try {
    Start-Sleep -Seconds 10
    if ($process.HasExited) { throw 'TUI test app exited.' }
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class ButtonsCliTuiSmokeWindow {
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern bool SetWindowText(IntPtr handle, string title);
}
'@
    $process.Refresh()
    $testTitle = "ButtonsCLI TUI Smoke $($process.Id)"
    if ($process.MainWindowHandle -eq 0 -or -not [ButtonsCliTuiSmokeWindow]::SetWindowText($process.MainWindowHandle, $testTitle)) {
        throw 'Could not identify the test-owned TUI window.'
    }
    & appsnap.exe -o ([System.IO.Path]::GetFullPath($CapturePath)) $testTitle
    if ($LASTEXITCODE -ne 0) { throw 'TUI capture failed.' }
    Write-Output "Isolated TUI capture: $CapturePath"
} finally {
    if (-not $process.HasExited) {
        if (-not $process.CloseMainWindow() -or -not $process.WaitForExit(5000)) { $process.Kill($true); $process.WaitForExit() }
    }
    $process.Dispose()
}
