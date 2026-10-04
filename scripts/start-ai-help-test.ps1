[CmdletBinding()]
param([switch]$SkipBuild)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$binaryPath = Join-Path $repoRoot 'target\debug\buttonscli.exe'
$fixtureHome = Join-Path ([IO.Path]::GetTempPath()) "buttonscli-ai-help-$([guid]::NewGuid().ToString('N'))"
$providerProcess = $null
$appProcess = $null
Push-Location $repoRoot
try {
    if (-not $SkipBuild) {
        & cargo build --bin buttonscli
        if ($LASTEXITCODE -ne 0) { throw 'The debug build failed.' }
    }
    if (-not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) { throw 'Build the native debug app first.' }
    $nodePath = (Get-Command node -ErrorAction Stop).Source
    $providerInfo = [Diagnostics.ProcessStartInfo]::new()
    $providerInfo.FileName = $nodePath
    $providerInfo.ArgumentList.Add((Join-Path $PSScriptRoot 'fake-ai-provider.mjs'))
    $providerInfo.UseShellExecute = $false
    $providerInfo.CreateNoWindow = $true
    $providerInfo.RedirectStandardOutput = $true
    $providerProcess = [Diagnostics.Process]::Start($providerInfo)
    $ready = $providerProcess.StandardOutput.ReadLineAsync()
    if (-not $ready.Wait(10000)) { throw 'Local fake provider did not become ready.' }
    $provider = $ready.Result | ConvertFrom-Json
    $profile = Join-Path $fixtureHome '.buttonscli-native\profiles\default'
    $roaming = Join-Path $fixtureHome 'AppData\Roaming'
    $local = Join-Path $fixtureHome 'AppData\Local'
    New-Item -ItemType Directory -Path $profile, $roaming, $local -Force | Out-Null
    @{
        schema_version = 1; revision = 1
        preferences = @{
            localization = @{ mode = 'manual'; manual_locale = 'en'; first_run_language_confirmed = $true }
            provider_settings = @{
                active_provider_id = 'local-fixture'
                providers = @(@{ id = 'local-fixture'; name = 'Local test fixture'; endpoint = $provider.endpoint; model = $provider.model })
            }
        }
    } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $profile 'native.json') -Encoding utf8NoBOM
    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $binaryPath
    $startInfo.WorkingDirectory = $fixtureHome
    $startInfo.UseShellExecute = $false
    $startInfo.Environment['USERPROFILE'] = $fixtureHome
    $startInfo.Environment['HOME'] = $fixtureHome
    $startInfo.Environment['APPDATA'] = $roaming
    $startInfo.Environment['LOCALAPPDATA'] = $local
    $startInfo.Environment['BUTTONSCLI_NATIVE_DISABLE_ACCOUNT'] = '1'
    $startInfo.Environment['BUTTONSCLI_NATIVE_DISABLE_REMOTE_CONFIG'] = '1'
    $startInfo.Environment['BUTTONSCLI_NATIVE_DEV_AI_HELP'] = '1'
    $startInfo.Environment['BUTTONSCLI_NATIVE_DEV_AI_AGENT'] = '0'
    $startInfo.Environment['BUTTONSCLI_NATIVE_DEV_REMOTE_CONTROL'] = '0'
    Write-Output "Opening an isolated debug workspace. Open AI Help and send any question; provider models also include slow, retry, quota, and malformed fixtures. Test profile: $fixtureHome"
    $appProcess = [Diagnostics.Process]::Start($startInfo)
    $appProcess.WaitForExit()
} finally {
    if ($null -ne $appProcess) { $appProcess.Dispose() }
    if ($null -ne $providerProcess) {
        if (-not $providerProcess.HasExited) { $providerProcess.Kill(); $providerProcess.WaitForExit() }
        $providerProcess.Dispose()
    }
    Pop-Location
}
