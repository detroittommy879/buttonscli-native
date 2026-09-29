[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [ValidateRange(10, 120)]
    [int]$StartupTimeoutSeconds = 30
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$binaryPath = Join-Path $repoRoot 'target\debug\buttonscli.exe'
$smokeHome = Join-Path ([System.IO.Path]::GetTempPath()) "buttonscli-native-control-live-$([guid]::NewGuid().ToString('N'))"
$appData = Join-Path $smokeHome 'AppData'
$roaming = Join-Path $appData 'Roaming'
$local = Join-Path $appData 'Local'
$controlDir = Join-Path $smokeHome '.buttonscli-native\control'
$process = $null
$ownedShellIds = @()
$cleanupFailure = $null

New-Item -ItemType Directory -Path $roaming, $local | Out-Null

function Invoke-ControlApi {
    param(
        [Parameter(Mandatory)][ValidateSet('GET', 'POST')][string]$Method,
        [Parameter(Mandatory)][string]$Path,
        [object]$Body
    )

    $request = @{
        Method = $Method
        Uri = "$script:baseUrl$Path"
        Headers = $script:headers
        TimeoutSec = 15
    }
    if ($null -ne $Body) {
        $request.ContentType = 'application/json'
        $request.Body = ConvertTo-Json -InputObject $Body -Depth 8 -Compress
    }
    Invoke-RestMethod @request
}

function Wait-ControlTabReady {
    param([Parameter(Mandatory)][string]$TabId)

    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    while ([DateTime]::UtcNow -lt $deadline) {
        $tab = (Invoke-ControlApi -Method GET -Path '/v1/tabs').tabs |
            Where-Object { $_.tabId -eq $TabId } |
            Select-Object -First 1
        if ($null -eq $tab) {
            throw "Control API did not list the expected tab $TabId."
        }
        if ($tab.exited) {
            throw "Test PTY $TabId exited before becoming ready."
        }
        if ($tab.ready) {
            return $tab
        }
        Start-Sleep -Milliseconds 100
    }
    throw "Timed out waiting for test PTY $TabId to become ready."
}

function Invoke-MarkerCommand {
    param(
        [Parameter(Mandatory)][string]$TabId,
        [Parameter(Mandatory)][string]$Marker
    )

    $selector = [Uri]::EscapeDataString($TabId)
    $payload = @{
        text = "Write-Output '$Marker'"
        waitForText = $Marker
        chars = 8192
        quietMs = 1200
        maxWaitMs = 12000
        intervalMs = 50
        enter = $true
    }
    $result = Invoke-ControlApi -Method POST -Path "/v1/tabs/$selector/run" -Body $payload
    if ($result.timedOut -or $result.completionReason -ne 'matched-text' -or
        -not $result.text.Contains($Marker)) {
        throw "PTY output observer did not capture marker '$Marker'; reason=$($result.completionReason), timeout=$($result.timedOut)."
    }

    $read = Invoke-ControlApi -Method GET -Path "/v1/tabs/$selector/read?chars=8192&from=bottom"
    if (-not $read.text.Contains($Marker)) {
        throw "Read API did not return PTY marker '$Marker'."
    }
    $result
}

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

    $shell = Get-Command pwsh.exe -ErrorAction SilentlyContinue
    if ($null -eq $shell) {
        $shell = Get-Command powershell.exe -ErrorAction Stop
    }
    $shellCommand = '"{0}" -NoLogo -NoProfile' -f $shell.Source
    $nativeRoot = Join-Path $smokeHome '.buttonscli-native'
    $null = New-Item -ItemType Directory -Path $nativeRoot -Force

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $binaryPath
    $startInfo.WorkingDirectory = $smokeHome
    $startInfo.UseShellExecute = $false
    $startInfo.Environment['USERPROFILE'] = $smokeHome
    $startInfo.Environment['HOME'] = $smokeHome
    $startInfo.Environment['APPDATA'] = $roaming
    $startInfo.Environment['LOCALAPPDATA'] = $local
    $startInfo.Environment['BUTTONSCLI_NATIVE_DISABLE_REMOTE_CONFIG'] = '1'
    $startInfo.Environment['BUTTONSCLI_NATIVE_DISABLE_ACCOUNT'] = '1'
    $startInfo.Environment['BUTTONSCLI_NATIVE_DEV_REMOTE_CONTROL'] = '1'

    $process = [System.Diagnostics.Process]::Start($startInfo)
    $descriptorPath = $null
    $deadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        $process.Refresh()
        if ($process.HasExited) {
            throw "Native GUI exited during startup with code $($process.ExitCode)"
        }
        $descriptorPath = Get-ChildItem -LiteralPath $controlDir -Filter '*.json' -File -ErrorAction SilentlyContinue |
            Select-Object -First 1 -ExpandProperty FullName
        if ($descriptorPath) { break }
        Start-Sleep -Milliseconds 150
    }
    if (-not $descriptorPath) {
        throw "Native control API did not publish a descriptor within $StartupTimeoutSeconds seconds."
    }

    $descriptor = Get-Content -LiteralPath $descriptorPath -Raw | ConvertFrom-Json
    $script:baseUrl = $descriptor.baseUrl
    $script:headers = @{ Authorization = "Bearer $($descriptor.authToken)" }
    $status = Invoke-ControlApi -Method GET -Path '/v1/status'
    if ($status.instanceId -ne $descriptor.instanceId) {
        throw 'Control status instance ID did not match its descriptor.'
    }

    $visibleBody = @{
        name = 'Control Live Visible'
        shell = $shellCommand
        cwd = $smokeHome
    }
    $visible = Invoke-ControlApi -Method POST -Path '/v1/tabs' -Body $visibleBody
    $visibleTabId = $visible.tab.tabId
    Wait-ControlTabReady $visibleTabId | Out-Null
    $visibleMarker = "BUTTONSCLI_VISIBLE_$([guid]::NewGuid().ToString('N'))"
    $null = Invoke-MarkerCommand $visibleTabId $visibleMarker

    $hiddenBody = @{
        name = 'Control Live Foreground'
        shell = $shellCommand
        cwd = $smokeHome
    }
    $hidden = Invoke-ControlApi -Method POST -Path '/v1/tabs' -Body $hiddenBody
    $hiddenTabId = $hidden.tab.tabId
    Wait-ControlTabReady $hiddenTabId | Out-Null
    $state = Invoke-ControlApi -Method GET -Path '/v1/tabs'
    $visibleState = $state.tabs | Where-Object { $_.tabId -eq $visibleTabId } | Select-Object -First 1
    if ($null -eq $visibleState -or $visibleState.isActive) {
        throw 'Creating a second tab did not move the first test PTY into the background.'
    }
    $hiddenMarker = "BUTTONSCLI_HIDDEN_$([guid]::NewGuid().ToString('N'))"
    $hiddenRun = Invoke-MarkerCommand $visibleTabId $hiddenMarker

    $ownedShellProcesses = @(
        Get-CimInstance Win32_Process -Filter "ParentProcessId = $($process.Id)" |
            Where-Object { $_.Name -in @('pwsh.exe', 'powershell.exe') }
    )
    if ($ownedShellProcesses.Count -lt 2) {
        throw "Expected two test-owned shell PTYs before shutdown; found $($ownedShellProcesses.Count)."
    }
    $ownedShellIds = @($ownedShellProcesses | ForEach-Object { [uint32]$_.ProcessId })

    Write-Output "Control API status authenticated for instance $($descriptor.instanceId)."
    Write-Output "Visible and background PTYs both captured unique output markers."
    Write-Output "Background run completion: $($hiddenRun.completionReason); timed out: $($hiddenRun.timedOut)."
}
finally {
    if ($null -ne $process -and -not $process.HasExited) {
        if (-not $process.CloseMainWindow() -or -not $process.WaitForExit(8000)) {
            $process.Kill($true)
            $process.WaitForExit()
        }
    }
    if ($null -ne $process) {
        $process.Refresh()
        if (-not $process.HasExited) {
            $process.Kill($true)
            $process.WaitForExit()
        }
        $process.Dispose()
    }
    if ($ownedShellIds.Count -gt 0) {
        Start-Sleep -Milliseconds 300
        $remainingShells = @($ownedShellIds | Where-Object {
            Get-Process -Id $_ -ErrorAction SilentlyContinue
        })
        if ($remainingShells.Count -gt 0) {
            $cleanupFailure = "PTY child process cleanup failed for PID(s): $($remainingShells -join ', ')"
        }
    }

    $resolvedTempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    $resolvedSmokeHome = [System.IO.Path]::GetFullPath($smokeHome)
    $withinTemp = $resolvedSmokeHome.StartsWith($resolvedTempRoot, [System.StringComparison]::OrdinalIgnoreCase)
    $expectedName = [System.IO.Path]::GetFileName($resolvedSmokeHome).StartsWith('buttonscli-native-control-live-', [System.StringComparison]::Ordinal)
    if ($withinTemp -and $expectedName -and (Test-Path -LiteralPath $resolvedSmokeHome)) {
        Remove-Item -LiteralPath $resolvedSmokeHome -Recurse -Force
    }
    elseif (Test-Path -LiteralPath $resolvedSmokeHome) {
        Write-Error "Refusing to remove unexpected smoke path: $resolvedSmokeHome"
    }
    Pop-Location
    if ($cleanupFailure) { throw $cleanupFailure }
}
