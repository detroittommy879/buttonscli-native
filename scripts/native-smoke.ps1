[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [string]$CapturePath,
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
$process = $null

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
    $startInfo.WorkingDirectory = $smokeHome
    $startInfo.UseShellExecute = $false
    $startInfo.Environment['USERPROFILE'] = $smokeHome
    $startInfo.Environment['HOME'] = $smokeHome
    $startInfo.Environment['APPDATA'] = $roaming
    $startInfo.Environment['LOCALAPPDATA'] = $local
    $startInfo.Environment['BUTTONSCLI_NATIVE_DISABLE_REMOTE_CONFIG'] = '1'
    $startInfo.Environment['BUTTONSCLI_NATIVE_DISABLE_ACCOUNT'] = '1'

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

    if (-not [string]::IsNullOrWhiteSpace($CapturePath)) {
        $appsnap = Get-Command appsnap.exe -ErrorAction Stop
        & $appsnap.Source -o $CapturePath $smokeWindowTitle
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
