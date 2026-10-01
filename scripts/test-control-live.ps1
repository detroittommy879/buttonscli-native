[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$WithMcpSdk,
    [switch]$WithLoadProbe,
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
$smokeError = $null

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
    foreach ($argument in @('--tabs', '3', '--shell', $shellCommand, '--cwd', $smokeHome,
        '--command', '1', "Add-Content -LiteralPath startup-one.txt -Value 'startup-one'",
        '--command', '3', "Add-Content -LiteralPath startup-three.txt -Value 'startup-three'")) {
        $startInfo.ArgumentList.Add($argument)
    }
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
    $startupDeadline = [DateTime]::UtcNow.AddSeconds(35)
    while ([DateTime]::UtcNow -lt $startupDeadline -and
        (-not (Test-Path -LiteralPath (Join-Path $smokeHome 'startup-one.txt')) -or
         -not (Test-Path -LiteralPath (Join-Path $smokeHome 'startup-three.txt')))) {
        Start-Sleep -Milliseconds 200
    }
    $startupTabs = (Invoke-ControlApi -Method GET -Path '/v1/tabs').tabs
    if ($startupTabs.Count -ne 3) { throw 'Startup did not create exactly three tabs.' }
    if ($startupTabs[1].lastInput) { throw 'Startup sent input to the tab without a command.' }
    foreach ($suffix in @('one', 'three')) {
        $startupFile = Join-Path $smokeHome "startup-$suffix.txt"
        if ((Get-Content -LiteralPath $startupFile -Raw).Trim() -ne "startup-$suffix") {
            throw "Startup command $suffix did not run exactly once in its requested working directory."
        }
    }
    Write-Output 'Startup created three tabs and ran each of two targeted commands once; the middle tab received no input.'
    if ($WithMcpSdk) {
        $sdkSmokePath = Join-Path $PSScriptRoot 'test-mcp-sdk.mjs'
        $sdkSmokeOutput = & $script:nodePath $sdkSmokePath --descriptor $descriptorPath 2>&1
        if ($LASTEXITCODE -ne 0) {
            throw "Official MCP SDK client failed against the live app: $(($sdkSmokeOutput | Out-String).Trim())"
        }
        Write-Output ($sdkSmokeOutput | Out-String).Trim()
    }

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

    $pacedMarker = "BUTTONSCLI_PACED_$([guid]::NewGuid().ToString('N'))"
    $null = Invoke-NativeCli -Arguments @(
        'send', '--tab', $visibleTabId, '--delivery', 'slow-typed', '--delay-ms', '1',
        '--text', "Write-Output '$pacedMarker'", '--enter', '--json'
    )
    $null = Invoke-NativeCli -Arguments @(
        'wait-for-text', '--tab', $visibleTabId, '--text', $pacedMarker,
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

    $exitingShellCommand = '"{0}" -NoLogo -NoProfile -Command "Start-Sleep -Seconds 2; exit"' -f $shell.Source
    $exitingTab = Invoke-ControlApi -Method POST -Path '/v1/tabs' -Body @{
        name = 'Control Live Exiting PTY'
        shell = $exitingShellCommand
        cwd = $smokeHome
    }
    $exitingTabId = $exitingTab.tab.tabId
    Wait-ControlTabReady $exitingTabId | Out-Null
    $slowPayload = 'x' * 80
    $slowJob = Start-Job -ArgumentList @(
        $script:nodePath, $script:cliHelperPath, $script:descriptorPath,
        $exitingTabId, $slowPayload
    ) -ScriptBlock {
        param($NodePath, $HelperPath, $InfoPath, $TabId, $Payload)
        $env:BUTTONSCLI_CONTROL_INFO_PATH = $InfoPath
        $result = & $NodePath $HelperPath send --tab $TabId --delivery slow-typed `
            --delay-ms 250 --text $Payload --json 2>&1
        [pscustomobject]@{
            exitCode = $LASTEXITCODE
            output = ($result | Out-String).Trim()
        }
    }
    try {
        $slowCompleted = Wait-Job -Job $slowJob -Timeout 10
        if ($null -eq $slowCompleted) {
            throw 'Slow-typed input did not stop after its PTY process exited.'
        }
        $slowResult = Receive-Job -Job $slowJob
        if ($slowResult.exitCode -eq 0 -or
            $slowResult.output -notmatch 'terminal process has exited|terminal closed before') {
            throw "Expected paced input to stop on PTY exit; got exit=$($slowResult.exitCode), output=$($slowResult.output)."
        }
    }
    finally {
        Stop-Job -Job $slowJob -ErrorAction SilentlyContinue
        Remove-Job -Job $slowJob -Force -ErrorAction SilentlyContinue
    }

    if ($WithLoadProbe) {
        foreach ($paneCount in @(1, 4, 10)) {
            # Keep emitted lines below even a narrow pane's physical width.
            # The compatibility output stream contains raw ConPTY wrap/redraw VT.
            $loadTag = "L$([guid]::NewGuid().ToString('N').Substring(0, 8))"
            if ($paneCount -eq 1) {
                $created = Invoke-ControlApi -Method POST -Path '/v1/tabs' -Body @{
                    name = "Load $paneCount"; shell = $shellCommand; cwd = $smokeHome
                }
                $loadTabs = @($created.tab)
            }
            else {
                $created = Invoke-ControlApi -Method POST -Path '/v1/layout/open' -Body @{
                    layout = 'grid'; columns = [int][Math]::Ceiling([Math]::Sqrt($paneCount))
                    names = @(1..$paneCount | ForEach-Object { "Load $paneCount pane $_" })
                    shell = $shellCommand; cwd = $smokeHome
                }
                $loadTabs = @($created.tabs)
            }
            if ($loadTabs.Count -ne $paneCount) { throw "Load probe did not open $paneCount terminals." }
            $baselines = @{}
            $markers = @{}
            foreach ($loadTab in $loadTabs) {
                $ready = Wait-ControlTabReady $loadTab.tabId
                # API readiness identifies a live PTY, not a ready shell prompt.
                $shellDeadline = [DateTime]::UtcNow.AddSeconds(20)
                do {
                    $prompt = Invoke-ControlApi -Method GET -Path "/v1/tabs/$($loadTab.tabId)/read?chars=4096"
                    if ($prompt.text -match 'PS [^\r\n]*>') { break }
                    Start-Sleep -Milliseconds 100
                } while ([DateTime]::UtcNow -lt $shellDeadline)
                if ($prompt.text -notmatch 'PS [^\r\n]*>') {
                    throw "Load probe shell did not reach its prompt on $($loadTab.tabId)."
                }
                $baselines[$loadTab.tabId] = $ready.outputSequence
                $markers[$loadTab.tabId] = "${loadTag}_$($loadTab.ptyId)"
            }
            $watch = [Diagnostics.Stopwatch]::StartNew()
            foreach ($loadTab in $loadTabs) {
                # The complete DONE marker never appears in echoed command text.
                $command = 'for ($i=1; $i -le 200; $i++) {{ Write-Output (''{0}:雪:'' + $i); Start-Sleep -Milliseconds 10 }}; Write-Output (''{0}'' + '':DONE'')' -f $markers[$loadTab.tabId]
                $null = Invoke-ControlApi -Method POST -Path "/v1/tabs/$($loadTab.tabId)/send" -Body @{
                    text = $command; enter = $true
                }
            }
            $remaining = @($loadTabs)
            while ($remaining.Count -gt 0 -and $watch.Elapsed.TotalSeconds -lt 45) {
                $unfinished = @()
                foreach ($loadTab in $remaining) {
                    $read = Invoke-ControlApi -Method GET -Path "/v1/tabs/$($loadTab.tabId)/read?chars=20000"
                    $marker = $markers[$loadTab.tabId]
                    if (-not $read.text.Contains("${marker}:DONE")) {
                        $unfinished += $loadTab
                        continue
                    }
                    for ($line = 1; $line -le 200; $line++) {
                        if (-not $read.text.Contains("${marker}:雪:$line`r`n")) {
                            throw "Load probe lost Unicode output line $line on $($loadTab.tabId)."
                        }
                    }
                    foreach ($otherMarker in $markers.Values) {
                        if ($otherMarker -ne $marker -and $read.text.Contains($otherMarker)) {
                            throw "Load output crossed into $($loadTab.tabId)."
                        }
                    }
                    if ($read.tab.ptyId -ne $loadTab.ptyId -or $read.tab.exited -or
                        $read.tab.outputSequence -le $baselines[$loadTab.tabId]) {
                        throw "Load probe changed or lost the live PTY $($loadTab.tabId)."
                    }
                }
                $remaining = @($unfinished)
                if ($remaining.Count -gt 0) { Start-Sleep -Milliseconds 100 }
            }
            if ($remaining.Count -gt 0) {
                foreach ($loadTab in $remaining) {
                    $diagnostic = Invoke-ControlApi -Method GET -Path "/v1/tabs/$($loadTab.tabId)/read?chars=20000"
                    # Only synthetic output from this test-owned profile is saved.
                    $diagnostic | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $repoRoot "target/load-probe-$($loadTab.tabId).json") -Encoding utf8
                    Write-Output "Load failure output for $($loadTab.tabId): $($diagnostic.text)"
                }
                throw "Load probe timed out with $($remaining.Count) unfinished terminals."
            }
            Write-Output "Load probe: $paneCount terminals, $($paneCount * 200) Unicode output lines verified in $($watch.ElapsedMilliseconds) ms."
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
    Write-Output 'Installed Node CLI exercised status, tabs, create, rename, read, waits, run, send (base64/file/stdin/paced), key, type-only preset, and grid layout.'
    Write-Output 'Paced delivery stopped with a clear error after its test PTY exited.'
    Write-Output 'Visible and background PTYs both captured unique output markers.'
    Write-Output "Background run completion: $($hiddenRun.completionReason); timed out: $($hiddenRun.timedOut)."
}
catch {
    $smokeError = $_
    throw
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
        # Windows may retain the shell's directory handle briefly after exit.
        $cleanupDeadline = [DateTime]::UtcNow.AddSeconds(5)
        while (Test-Path -LiteralPath $resolvedSmokeHome) {
            try {
                Remove-Item -LiteralPath $resolvedSmokeHome -Recurse -Force
                break
            }
            catch {
                if ([DateTime]::UtcNow -ge $cleanupDeadline) {
                    $cleanupFailure = "Could not remove test profile ${resolvedSmokeHome}: $($_.Exception.Message)"
                    break
                }
                Start-Sleep -Milliseconds 100
            }
        }
    }
    elseif (Test-Path -LiteralPath $resolvedSmokeHome) {
        Write-Error "Refusing to remove unexpected smoke path: $resolvedSmokeHome"
    }
    Pop-Location
    if ($cleanupFailure) {
        if ($null -ne $smokeError) { Write-Warning $cleanupFailure }
        else { throw $cleanupFailure }
    }
}
