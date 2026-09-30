[CmdletBinding()]
param([ValidateRange(10, 60)][int]$StartupTimeoutSeconds = 30)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$root = Join-Path ([IO.Path]::GetTempPath()) "buttonscli-close-paste-$([guid]::NewGuid().ToString('N'))"
$roaming = Join-Path $root 'AppData\Roaming'
$local = Join-Path $root 'AppData\Local'
$control = Join-Path $root '.buttonscli-native\control'
$process = $null
$job = $null
New-Item -ItemType Directory -Path $roaming, $local | Out-Null

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class ButtonsCliClosePasteWindow {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr data);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint x, uint y, uint data, UIntPtr extra);
    public delegate bool EnumWindowsProc(IntPtr hwnd, IntPtr data);
    public static IntPtr MainWindow(int processId) {
        IntPtr found=IntPtr.Zero;
        EnumWindows((hwnd,data)=>{ uint owner; GetWindowThreadProcessId(hwnd,out owner); if(owner==(uint)processId){var title=new StringBuilder(256);GetWindowText(hwnd,title,256);if(title.ToString()=="ButtonsCLI"){found=hwnd;return false;}}return true;},IntPtr.Zero);
        return found;
    }
}
'@

try {
    $binary = Join-Path $repoRoot 'target\debug\buttonscli.exe'
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw 'Build target\debug\buttonscli.exe before running this GUI acceptance check.'
    }
    $shell = (Get-Command pwsh.exe -ErrorAction SilentlyContinue).Source
    if (-not $shell) { $shell = (Get-Command powershell.exe -ErrorAction Stop).Source }

    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $binary
    $start.WorkingDirectory = $root
    $start.UseShellExecute = $false
    $start.Environment['USERPROFILE'] = $root
    $start.Environment['HOME'] = $root
    $start.Environment['APPDATA'] = $roaming
    $start.Environment['LOCALAPPDATA'] = $local
    $start.Environment['BUTTONSCLI_NATIVE_DISABLE_REMOTE_CONFIG'] = '1'
    $start.Environment['BUTTONSCLI_NATIVE_DISABLE_ACCOUNT'] = '1'
    $start.Environment['BUTTONSCLI_NATIVE_DEV_REMOTE_CONTROL'] = '1'
    $process = [Diagnostics.Process]::Start($start)

    $deadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
    do {
        $process.Refresh()
        if ($process.HasExited) { throw "Test app exited with code $($process.ExitCode)." }
        $descriptorPath = Get-ChildItem -LiteralPath $control -Filter '*.json' -File -ErrorAction SilentlyContinue |
            Select-Object -First 1 -ExpandProperty FullName
        if (-not $descriptorPath) { Start-Sleep -Milliseconds 150 }
    } while (-not $descriptorPath -and [DateTime]::UtcNow -lt $deadline)
    if (-not $descriptorPath) { throw 'Timed out waiting for the isolated control descriptor.' }

    $descriptor = Get-Content -LiteralPath $descriptorPath -Raw | ConvertFrom-Json
    $headers = @{ Authorization = "Bearer $($descriptor.authToken)" }
    $base = $descriptor.baseUrl

    $window = [ButtonsCliClosePasteWindow]::MainWindow($process.Id)
    if ($window -eq [IntPtr]::Zero) { throw 'Could not find the test-owned main window.' }
    $rect = New-Object ButtonsCliClosePasteWindow+Rect
    [ButtonsCliClosePasteWindow]::GetWindowRect($window, [ref]$rect) | Out-Null
    [ButtonsCliClosePasteWindow]::SetForegroundWindow($window) | Out-Null
    [ButtonsCliClosePasteWindow]::SetCursorPos(($rect.Left + 540), ($rect.Top + 598)) | Out-Null
    [ButtonsCliClosePasteWindow]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 120
    [ButtonsCliClosePasteWindow]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Seconds 2
    $shellCommand = '"{0}" -NoLogo -NoProfile' -f $shell
    $create = {
        param($Name)
        Invoke-RestMethod -Method Post -Uri "$base/v1/tabs" -Headers $headers `
            -ContentType 'application/json' -Body (ConvertTo-Json @{ name=$Name; shell=$shellCommand; cwd=$root } -Compress)
    }
    $receiver = & $create 'Paste race receiver'
    $target = & $create 'Paste race target'
    $targetId = $target.tab.tabId
    $receiverId = $receiver.tab.tabId
    $payload = ('x' * 32) + 'CLOSE_PASTE_MUST_NOT_REACH_RECEIVER'
    $job = Start-Job -ArgumentList @($base, $headers, $targetId, $payload) -ScriptBlock {
        param($BaseUrl, $AuthHeaders, $TabId, $Text)
        try {
            Invoke-RestMethod -Method Post -Uri "$BaseUrl/v1/tabs/$TabId/send" `
                -Headers $AuthHeaders -ContentType 'application/json' `
                -Body (ConvertTo-Json @{ text=$Text; delivery='slow-typed'; delayMs=250 } -Compress) -TimeoutSec 45
            'unexpected-success'
        } catch {
            $_.ErrorDetails.Message
        }
    }
    Start-Sleep -Seconds 2

    Add-Type -AssemblyName System.Windows.Forms
    $window = [ButtonsCliClosePasteWindow]::MainWindow($process.Id)
    if ($window -eq [IntPtr]::Zero) { throw 'Could not find the test-owned main window.' }
    [ButtonsCliClosePasteWindow]::SetForegroundWindow($window) | Out-Null
    Start-Sleep -Milliseconds 250
    [Windows.Forms.SendKeys]::SendWait('^+w')

    $completed = Wait-Job -Job $job -Timeout 15
    if (-not $completed) { throw 'Paced input did not stop after the target tab closed.' }
    $sendResult = Receive-Job -Job $job | Out-String
    if ($sendResult -match 'unexpected-success' -or $sendResult -notmatch 'closed before|not found') {
        throw "Expected a closed-target error from paced input; got: $sendResult"
    }
    $tabs = Invoke-RestMethod -Method Get -Uri "$base/v1/tabs" -Headers $headers
    if ($tabs.tabs.tabId -contains $targetId) { throw 'The target tab remained open after Ctrl+Shift+W.' }
    $receiverText = (Invoke-RestMethod -Method Get -Uri "$base/v1/tabs/$receiverId/read?lines=100" -Headers $headers).text
    if ($receiverText.Contains('CLOSE_PASTE_MUST_NOT_REACH_RECEIVER')) {
        throw 'Paced input was delivered to the other tab after its original target closed.'
    }
    Write-Output 'Closing the target tab stopped the paced send with a closed-target error.'
    Write-Output 'The remaining tab did not receive the target payload.'
}
finally {
    if ($null -ne $job) { Stop-Job $job -ErrorAction SilentlyContinue; Remove-Job $job -Force -ErrorAction SilentlyContinue }
    if ($null -ne $process -and -not $process.HasExited) {
        $process.CloseMainWindow() | Out-Null
        if (-not $process.WaitForExit(5000)) { $process.Kill($true); $process.WaitForExit() }
    }
    if ($null -ne $process) { $process.Dispose() }
    $tempBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\', '/')
    $resolvedRoot = [IO.Path]::GetFullPath($root)
    if ($resolvedRoot.StartsWith($tempBase + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -and
        [IO.Path]::GetFileName($resolvedRoot).StartsWith('buttonscli-close-paste-', [StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $resolvedRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
