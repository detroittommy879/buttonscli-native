# Run with Windows PowerShell 5.1 for its UI Automation assemblies.
[CmdletBinding()]
param([Parameter(Mandatory)][long]$WindowHandle, [Parameter(Mandatory)][int]$AppProcessId, [string]$CapturePath)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$root = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$WindowHandle)
$controlCondition = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::NameProperty, 'Agent Inst.')
$controlDeadline = [DateTime]::UtcNow.AddSeconds(8)
do {
    $control = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $controlCondition)
    if ($null -ne $control -and $control.Current.IsEnabled) { break }
    Start-Sleep -Milliseconds 100
} while ([DateTime]::UtcNow -lt $controlDeadline)
if ($null -eq $control -or -not $control.Current.IsEnabled) { throw 'Local JSON override did not enable the control handoff button.' }
$buttonCondition = [System.Windows.Automation.AndCondition]::new(
    [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty, 'AI Help'),
    [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Button))
$button = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $buttonCondition)
if ($null -eq $button) { throw 'AI Help button is absent.' }
$button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
$windowCondition = [System.Windows.Automation.AndCondition]::new(
    [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty, 'AI Help'),
    [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $AppProcessId))
$desktop = [System.Windows.Automation.AutomationElement]::RootElement
$deadline = [DateTime]::UtcNow.AddSeconds(10)
do {
    $chat = $desktop.FindFirst([System.Windows.Automation.TreeScope]::Children, $windowCondition)
    if ($null -ne $chat) { break }
    Start-Sleep -Milliseconds 100
} while ([DateTime]::UtcNow -lt $deadline)
if ($null -eq $chat) { throw 'AI Help did not open its separate window.' }
if (-not [string]::IsNullOrWhiteSpace($CapturePath)) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class ButtonsCliChatSmokeWindow {
    private delegate bool EnumProc(IntPtr handle, IntPtr parameter);
    [DllImport("user32.dll")] private static extern bool EnumWindows(EnumProc callback, IntPtr parameter);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr handle, out uint processId);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetWindowText(IntPtr handle, StringBuilder text, int count);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern bool SetWindowText(IntPtr handle, string text);
    public static bool RenameOwnedChat(int processId, string title) {
        bool renamed = false;
        EnumWindows((handle, parameter) => {
            uint owner;
            GetWindowThreadProcessId(handle, out owner);
            if (owner != (uint)processId) return true;
            var name = new StringBuilder(256);
            GetWindowText(handle, name, name.Capacity);
            if (name.ToString() != "AI Help") return true;
            renamed = SetWindowText(handle, title);
            return false;
        }, IntPtr.Zero);
        return renamed;
    }
}
'@
    $captureTitle = "ButtonsCLI AI Help Smoke $AppProcessId"
    if (-not [ButtonsCliChatSmokeWindow]::RenameOwnedChat($AppProcessId, $captureTitle)) { throw 'Could not identify the test-owned chat window for capture.' }
    Start-Sleep -Seconds 1
    & appsnap.exe -o $CapturePath $captureTitle
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $CapturePath)) { throw 'AI Help capture failed.' }
}
# Child-viewport UIA controls currently expose a placeholder tree. Do not claim
# composer accessibility here; optimized app/PTY tests verify the execution gates.
Write-Output 'Local JSON override: enabled control handoff and separate AI Help window pass without debug environment flags.'
