# Run only against a test-owned process/profile from native-smoke.ps1.
[CmdletBinding()]
param([Parameter(Mandatory)][long]$WindowHandle, [Parameter(Mandatory)][int]$AppProcessId,
      [Parameter(Mandatory)][string]$ProfilePath, [string]$CapturePath)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class ButtonsCliSettingsProbe {
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr handle, IntPtr after, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr handle, uint message, IntPtr w, IntPtr l);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool SetWindowText(IntPtr handle, string title);
}
'@
$root = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$WindowHandle)
$desktop = [System.Windows.Automation.AutomationElement]::RootElement
$condition = [System.Windows.Automation.AndCondition]::new(
    [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty, 'ButtonsCLI Settings'),
    [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $AppProcessId))
function Open-OwnedSettings {
    $buttonCondition = [System.Windows.Automation.AndCondition]::new(
        [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty, 'Settings'),
        [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Button))
    $button = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $buttonCondition)
    if ($null -eq $button) { throw 'Settings status button is absent.' }
    $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    $deadline = [DateTime]::UtcNow.AddSeconds(8)
    do {
        $window = $desktop.FindFirst([System.Windows.Automation.TreeScope]::Children, $condition)
        if ($null -ne $window) { return $window }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw 'Settings did not open its separate window.'
}
$window = Open-OwnedSettings
$handle = [IntPtr]$window.Current.NativeWindowHandle
if (-not [ButtonsCliSettingsProbe]::SetWindowPos($handle, [IntPtr]::Zero, 80, 80, 1100, 800, 4)) { throw 'Settings resize failed.' }
Start-Sleep -Milliseconds 1200
$before = $window.Current.BoundingRectangle
[void][ButtonsCliSettingsProbe]::PostMessage($handle, 0x10, [IntPtr]::Zero, [IntPtr]::Zero)
$deadline = [DateTime]::UtcNow.AddSeconds(8)
do {
    Start-Sleep -Milliseconds 100
    $settings = Get-Content -LiteralPath $ProfilePath -Raw | ConvertFrom-Json
    $saved = $settings.preferences.settings_layout
    if ($null -ne $saved -and $null -ne $saved.position) { break }
} while ([DateTime]::UtcNow -lt $deadline)
if ($null -eq $saved -or $null -eq $saved.position) { throw 'Closing Settings did not persist its geometry.' }
$window = Open-OwnedSettings
Start-Sleep -Milliseconds 1000
$after = $window.Current.BoundingRectangle
if ([Math]::Abs($after.Width - $before.Width) -gt 3 -or [Math]::Abs($after.Height - $before.Height) -gt 3 -or
    [Math]::Abs($after.Left - $before.Left) -gt 3 -or [Math]::Abs($after.Top - $before.Top) -gt 3) {
    throw "Settings geometry changed after reopening: before $before, after $after."
}
if (-not [string]::IsNullOrWhiteSpace($CapturePath)) {
    $title = "ButtonsCLI Settings Smoke $AppProcessId"
    [void][ButtonsCliSettingsProbe]::SetWindowText([IntPtr]$window.Current.NativeWindowHandle, $title)
    & appsnap.exe -o $CapturePath $title
    if ($LASTEXITCODE -ne 0) { throw 'Settings capture failed.' }
}
[void][ButtonsCliSettingsProbe]::PostMessage([IntPtr]$window.Current.NativeWindowHandle, 0x10, [IntPtr]::Zero, [IntPtr]::Zero)
Write-Output "Windows Settings: resize/move, close persistence and reopen geometry pass ($($saved.size -join 'x') points)."
