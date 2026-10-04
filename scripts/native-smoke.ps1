[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$Release,
    [switch]$WithLocalFeatureFlags,
    [string]$AiHelpCapturePath,
    [switch]$WithThemeControls,
    [switch]$WithAbruptExit,
    [switch]$WithAccessibility,
    [switch]$WithSettingsLayout,
    [string]$SettingsCapturePath,
    [string]$CapturePath,
    [ValidateRange(5, 120)]
    [int]$StartupTimeoutSeconds = 30
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$buildProfile = if ($Release) { 'release' } else { 'debug' }
$binaryPath = Join-Path $repoRoot "target\$buildProfile\buttonscli.exe"
$smokeHome = Join-Path ([System.IO.Path]::GetTempPath()) "buttonscli-native-smoke-$([guid]::NewGuid().ToString('N'))"
$appData = Join-Path $smokeHome 'AppData'
$roaming = Join-Path $appData 'Roaming'
$local = Join-Path $appData 'Local'
$process = $null
$trackedShells = @()

New-Item -ItemType Directory -Path $roaming, $local -Force | Out-Null

if ($WithThemeControls -or $WithAccessibility -or $WithLocalFeatureFlags -or $WithSettingsLayout) {
    # Seed only the test-owned profile; leave the user's settings untouched.
    $fixtureProfile = Join-Path $smokeHome '.buttonscli-native\profiles\default'
    New-Item -ItemType Directory -Path $fixtureProfile -Force | Out-Null
    @{
        schema_version = 1
        revision = 1
        preferences = @{
            localization = @{ mode = 'manual'; manual_locale = 'en'; first_run_language_confirmed = $true }
            favorite_theme_ids = @('basic2')
        }
    } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $fixtureProfile 'native.json') -Encoding utf8NoBOM
}

if ($WithLocalFeatureFlags) {
    @{ enable_all = $true } | ConvertTo-Json |
        Set-Content -LiteralPath (Join-Path $smokeHome '.buttonscli-native\feature-flags.json') -Encoding utf8NoBOM
}

Push-Location $repoRoot
try {
    if (-not $SkipBuild) {
        $buildArguments = @('build', '--locked', '--bin', 'buttonscli')
        if ($Release) { $buildArguments += '--release' }
        & cargo @buildArguments
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build --bin buttonscli failed with exit code $LASTEXITCODE"
        }
    }

    if (-not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
        throw "Native executable not found: $binaryPath"
    }

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
    if ($WithLocalFeatureFlags) {
        foreach ($flagName in @('AI_HELP', 'AI_AGENT', 'THEME_GENERATOR', 'QUICK_SECRETS', 'REMOTE_CONTROL')) {
            $startInfo.Environment.Remove("BUTTONSCLI_NATIVE_DEV_$flagName") | Out-Null
        }
    }

    if (-not ('ButtonsCliSmokeWindow' -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class ButtonsCliSmokeWindow {
    [StructLayout(LayoutKind.Sequential)]
    public struct NativeRect {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern bool SetWindowText(IntPtr hWnd, string text);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool IsWindowVisible(IntPtr hWnd);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool GetWindowRect(IntPtr hWnd, out NativeRect rect);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr lParam);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int maxCount);

    private delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    public static IntPtr FindButtonsCliWindow(int processId) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((hWnd, _) => {
            GetWindowThreadProcessId(hWnd, out uint ownerProcessId);
            if (ownerProcessId != (uint)processId || !IsWindowVisible(hWnd)) {
                return true;
            }
            var title = new StringBuilder(256);
            GetWindowText(hWnd, title, title.Capacity);
            if (title.ToString() == "ButtonsCLI") {
                found = hWnd;
                return false;
            }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
'@
    }

    $process = [System.Diagnostics.Process]::Start($startInfo)
    $deadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
    $windowFound = $false
    $windowHandle = [IntPtr]::Zero

    while ([DateTime]::UtcNow -lt $deadline) {
        $process.Refresh()
        if ($process.HasExited) {
            throw "Native GUI exited during startup with code $($process.ExitCode)"
        }
        $windowHandle = [ButtonsCliSmokeWindow]::FindButtonsCliWindow($process.Id)
        if ($windowHandle -ne [IntPtr]::Zero) {
            $windowFound = $true
            break
        }
        Start-Sleep -Milliseconds 250
    }

    if (-not $windowFound) {
        throw "Native GUI did not create a main window within $StartupTimeoutSeconds seconds"
    }

    $smokeWindowTitle = "ButtonsCLI Native Smoke $($process.Id)"
    if (-not [ButtonsCliSmokeWindow]::SetWindowText($windowHandle, $smokeWindowTitle)) {
        throw "Could not assign a unique title to the test-owned native window"
    }

    $windowRect = [ButtonsCliSmokeWindow+NativeRect]::new()
    if (-not [ButtonsCliSmokeWindow]::IsWindowVisible($windowHandle) -or
        -not [ButtonsCliSmokeWindow]::GetWindowRect($windowHandle, [ref]$windowRect)) {
        throw "Native GUI window is not visible or has no screen bounds"
    }
    $windowWidth = $windowRect.Right - $windowRect.Left
    $windowHeight = $windowRect.Bottom - $windowRect.Top
    if ($windowWidth -lt 500 -or $windowHeight -lt 350) {
        throw "Native GUI window is unexpectedly small ($($windowWidth)x$($windowHeight))"
    }

    Start-Sleep -Seconds 2

    if ($WithLocalFeatureFlags) {
        $windowsPowerShell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
        & $windowsPowerShell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'probe-local-feature-flags.ps1') -WindowHandle $windowHandle.ToInt64() -AppProcessId $process.Id -CapturePath $AiHelpCapturePath
        if ($LASTEXITCODE -ne 0) { throw 'Native local feature flags UI acceptance failed.' }
    }

    if ($WithAccessibility) {
        $windowsPowerShell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
        & $windowsPowerShell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'probe-accessibility.ps1') -WindowHandle $windowHandle.ToInt64()
        if ($LASTEXITCODE -ne 0) { throw 'Native Windows UI Automation acceptance failed.' }
    }

    if ($WithSettingsLayout) {
        $windowsPowerShell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
        & $windowsPowerShell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'probe-settings-layout.ps1') -WindowHandle $windowHandle.ToInt64() -AppProcessId $process.Id -ProfilePath (Join-Path $fixtureProfile 'native.json') -CapturePath $SettingsCapturePath
        if ($LASTEXITCODE -ne 0) { throw 'Native Settings layout acceptance failed.' }
    }

    if (-not [string]::IsNullOrWhiteSpace($CapturePath)) {
        $appsnap = Get-Command appsnap.exe -ErrorAction SilentlyContinue
        if ($null -ne $appsnap) {
            & $appsnap.Source -o $CapturePath $smokeWindowTitle
        } else {
            $uvx = Get-Command uvx -ErrorAction Stop
            & $uvx.Source appsnap -o $CapturePath $smokeWindowTitle
        }
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $CapturePath -PathType Leaf)) {
            throw "appsnap could not capture the test-owned native window"
        }
        $screenshot = [System.Drawing.Image]::FromFile($CapturePath)
        $screenshotWidth = $screenshot.Width
        $screenshotHeight = $screenshot.Height
        $screenshot.Dispose()
        if ($screenshotWidth -lt 500 -or $screenshotHeight -lt 350) {
            throw "Startup screenshot is unexpectedly small ($($screenshotWidth)x$($screenshotHeight))"
        }
        Write-Output "Startup screenshot: $CapturePath"
    }

    Write-Output "Native GUI opened and stayed alive (PID $($process.Id))."
    Write-Output "Window bounds: ${windowWidth}x${windowHeight}px"
    Write-Output "Unique smoke window title: $smokeWindowTitle"
    Write-Output "Isolated native data root: $smokeHome\.buttonscli-native"
    if ($WithAbruptExit) {
        $trackedShells = @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $($process.Id)" |
            Where-Object { $_.Name -in @('cmd.exe', 'powershell.exe', 'pwsh.exe') } |
            ForEach-Object { [System.Diagnostics.Process]::GetProcessById([int]$_.ProcessId) })
        if ($trackedShells.Count -eq 0) {
            throw 'No test-owned shell was found for abrupt-exit verification.'
        }
        # Hold handles before exit, so PID reuse cannot retarget this check.
        foreach ($shell in $trackedShells) { $null = $shell.Handle }
        $process.Kill($false) # Only the host: jobs must clean up the shells.
        $process.WaitForExit()
        foreach ($shell in $trackedShells) {
            if (-not $shell.WaitForExit(5000)) {
                throw "Test-owned shell survived abrupt host exit: $($shell.Id)"
            }
        }
        Write-Output "Abrupt host exit cleaned up $($trackedShells.Count) test-owned shell(s)."
    }
}
finally {
    foreach ($shell in $trackedShells) {
        if (-not $shell.HasExited) { $shell.Kill($true); $shell.WaitForExit() }
        $shell.Dispose()
    }
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
