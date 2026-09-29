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

function Invoke-NativeCli {
    param(
        [Parameter(Mandatory)][string[]]$Arguments,
        [string]$StdinText
    )

    $previousInfoPath = $env:BUTTONSCLI_CONTROL_INFO_PATH
    $env:BUTTONSCLI_CONTROL_INFO_PATH = $script:descriptorPath
    try {
        if ($PSBoundParameters.ContainsKey('StdinText')) {
            $output = $StdinText | & $script:nodePath $script:cliHelperPath @Arguments 2>&1
        }
        else {
            $output = & $script:nodePath $script:cliHelperPath @Arguments 2>&1
        }
        $exitCode = $LASTEXITCODE
        $outputText = ($output | Out-String).Trim()
        if ($exitCode -ne 0) {
            throw "buttonsclictl $($Arguments[0]) failed with exit code ${exitCode}: $outputText"
        }
        if ($Arguments -contains '--json') {
            return ConvertFrom-Json -InputObject $outputText
        }
        return $outputText
    }
    finally {
        if ($null -eq $previousInfoPath) {
            Remove-Item Env:BUTTONSCLI_CONTROL_INFO_PATH -ErrorAction SilentlyContinue
        }
        else {
            $env:BUTTONSCLI_CONTROL_INFO_PATH = $previousInfoPath
        }
    }
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
    $script:descriptorPath = $descriptorPath
    $script:baseUrl = $descriptor.baseUrl
    $script:headers = @{ Authorization = "Bearer $($descriptor.authToken)" }
    $node = Get-Command node.exe -ErrorAction Stop
    $script:nodePath = $node.Source
    $helperDir = Join-Path $nativeRoot 'helpers'
    $script:cliHelperPath = Get-ChildItem -LiteralPath $helperDir -Filter 'buttonsclictl-*.mjs' -File |
        Select-Object -First 1 -ExpandProperty FullName
    if (-not $script:cliHelperPath) {
        throw 'Native control server did not install the pinned Node CLI helper.'
    }

    $status = Invoke-ControlApi -Method GET -Path '/v1/status'
    if ($status.instanceId -ne $descriptor.instanceId) {
        throw 'Control status instance ID did not match its descriptor.'
    }
    $cliStatus = Invoke-NativeCli -Arguments @('status', '--json')
    if ($cliStatus.instanceId -ne $descriptor.instanceId) {
        throw 'Installed Node CLI did not connect to the selected test instance.'
    }
    $null = Invoke-NativeCli -Arguments @('tabs', '--json')

    $visible = Invoke-NativeCli -Arguments @(
        'create-tab', '--name', 'Control Live Visible', '--shell', $shellCommand,
        '--cwd', $smokeHome, '--json'
    )
    $visibleTabId = $visible.tab.tabId
    Wait-ControlTabReady $visibleTabId | Out-Null
    $renamed = Invoke-NativeCli -Arguments @(
        'rename-tab', '--tab', $visibleTabId, '--name', 'Control Live Visible Renamed', '--json'
    )
    if ($renamed.tab.title -ne 'Control Live Visible Renamed') {
        throw 'Installed Node CLI did not rename the test tab.'
    }

    $visibleMarker = "BUTTONSCLI_VISIBLE_$([guid]::NewGuid().ToString('N'))"
    $visibleRun = Invoke-NativeCli -Arguments @(
        'run', '--tab', $visibleTabId, '--text', "Write-Output '$visibleMarker'",
        '--wait-for-text', $visibleMarker, '--chars', '8192', '--timeout-ms', '12000',
        '--interval-ms', '50', '--json'
    )
    if ($visibleRun.timedOut -or $visibleRun.completionReason -ne 'matched-text' -or
        -not $visibleRun.text.Contains($visibleMarker)) {
        throw "Installed Node CLI did not capture visible PTY output marker '$visibleMarker'."
    }
    $read = Invoke-NativeCli -Arguments @('read', '--tab', $visibleTabId, '--lines', '120', '--json')
    if (-not $read.text.Contains($visibleMarker)) {
        throw 'Installed Node CLI read did not return the visible PTY marker.'
    }
    $null = Invoke-NativeCli -Arguments @(
        'wait-for-text', '--tab', $visibleTabId, '--text', $visibleMarker,
        '--timeout-ms', '2500', '--interval-ms', '50', '--json'
    )
    $null = Invoke-NativeCli -Arguments @(
        'wait-for-quiet', '--tab', $visibleTabId, '--quiet-ms', '200',
        '--timeout-ms', '5000', '--interval-ms', '50', '--json'
    )

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
    $hiddenRun = Invoke-NativeCli -Arguments @(
        'run', '--tab', $visibleTabId, '--text', "Write-Output '$hiddenMarker'",
        '--wait-for-text', $hiddenMarker, '--chars', '8192', '--timeout-ms', '12000',
        '--interval-ms', '50', '--json'
    )
    if ($hiddenRun.timedOut -or $hiddenRun.completionReason -ne 'matched-text' -or
        -not $hiddenRun.text.Contains($hiddenMarker)) {
        throw "Installed Node CLI did not capture background PTY output marker '$hiddenMarker'."
    }
    $null = Invoke-NativeCli -Arguments @('presets', '--json')

    $base64Marker = "BUTTONSCLI_BASE64_$([guid]::NewGuid().ToString('N'))"
    $base64Command = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes("Write-Output '$base64Marker'"))
    $null = Invoke-NativeCli -Arguments @(
        'send', '--tab', $visibleTabId, '--base64', $base64Command, '--enter', '--json'
    )
    $null = Invoke-NativeCli -Arguments @(
        'wait-for-text', '--tab', $visibleTabId, '--text', $base64Marker,
        '--timeout-ms', '5000', '--interval-ms', '50', '--json'
    )

    $fileMarker = "BUTTONSCLI_FILE_$([guid]::NewGuid().ToString('N'))"
    $payloadFile = Join-Path $smokeHome 'cli-payload.txt'
    [System.IO.File]::WriteAllText(
        $payloadFile,
        "Write-Output '$fileMarker'",
        [System.Text.UTF8Encoding]::new($false)
    )
    $null = Invoke-NativeCli -Arguments @(
        'send', '--tab', $visibleTabId, '--file', $payloadFile, '--enter', '--json'
    )
    $null = Invoke-NativeCli -Arguments @(
        'wait-for-text', '--tab', $visibleTabId, '--text', $fileMarker,
        '--timeout-ms', '5000', '--interval-ms', '50', '--json'
    )

    $stdinMarker = "BUTTONSCLI_STDIN_$([guid]::NewGuid().ToString('N'))"
    $stdinCommand = "Write-Output '$stdinMarker'"
    $null = Invoke-NativeCli -Arguments @(
        'send', '--tab', $visibleTabId, '--stdin', '--enter', '--json'
    ) -StdinText $stdinCommand
    $null = Invoke-NativeCli -Arguments @(
        'wait-for-text', '--tab', $visibleTabId, '--text', $stdinMarker,
        '--timeout-ms', '5000', '--interval-ms', '50', '--json'
    )

    $presets = Invoke-NativeCli -Arguments @('presets', '--json')
    $typeOnlyPreset = $presets.presets |
        Where-Object { $_.label -eq 'SSH Template' -and -not $_.sendEnter } |
        Select-Object -First 1
    if ($null -eq $typeOnlyPreset) {
        throw 'Default type-only SSH Template preset was missing from the test profile.'
    }
    $null = Invoke-NativeCli -Arguments @(
        'preset-run', '--label', $typeOnlyPreset.label, '--tab', $visibleTabId, '--json'
    )
    $typeOnlyState = (Invoke-ControlApi -Method GET -Path '/v1/tabs').tabs |
        Where-Object { $_.tabId -eq $visibleTabId } |
        Select-Object -First 1
    if ($typeOnlyState.lastInput -ne $typeOnlyPreset.command) {
        throw 'Type-only preset did not leave its literal command at the PTY input cursor.'
    }

    $null = Invoke-NativeCli -Arguments @('key', 'ctrl+c', '--tab', $visibleTabId, '--json')

    $layout = Invoke-NativeCli -Arguments @(
        'open-layout', '--layout', 'grid', '--name', 'Control CLI Grid A',
        '--name', 'Control CLI Grid B', '--columns', '2', '--shell', $shellCommand,
        '--cwd', $smokeHome, '--json'
    )
    if (-not $layout.ok -or $layout.layout -ne 'grid' -or $layout.columns -ne 2 -or
        $layout.tabs.Count -ne 2 -or $layout.visibleTabIds.Count -ne 2) {
        throw 'Installed Node CLI did not open the requested two-tab grid layout.'
    }
    foreach ($layoutTab in $layout.tabs) {
        Wait-ControlTabReady $layoutTab.tabId | Out-Null
        if (-not ($layout.visibleTabIds -contains $layoutTab.tabId)) {
            throw "Grid tab $($layoutTab.tabId) was not included in the visible pane mapping."
        }
    }
    $cliTabs = Invoke-NativeCli -Arguments @('tabs', '--json')
    foreach ($layoutTab in $layout.tabs) {
        if (-not ($cliTabs.tabs | Where-Object { $_.tabId -eq $layoutTab.tabId })) {
            throw "Installed Node CLI did not list layout tab $($layoutTab.tabId)."
        }
    }

    $ownedShellProcesses = @(
        Get-CimInstance Win32_Process -Filter "ParentProcessId = $($process.Id)" |
            Where-Object { $_.Name -in @('pwsh.exe', 'powershell.exe') }
    )
    if ($ownedShellProcesses.Count -lt 2) {
        throw "Expected two test-owned shell PTYs before shutdown; found $($ownedShellProcesses.Count)."
    }
    $ownedShellIds = @($ownedShellProcesses | ForEach-Object { [uint32]$_.ProcessId })

    Write-Output "Control API status authenticated for instance $($descriptor.instanceId)."
    Write-Output 'Installed Node CLI exercised status, tabs, create, rename, read, waits, run, send (base64/file/stdin), key, type-only preset, and grid layout.'
    Write-Output 'Visible and background PTYs both captured unique output markers.'
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
