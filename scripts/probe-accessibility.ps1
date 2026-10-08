# Run with Windows PowerShell 5.1, which includes the UI Automation assemblies.
[CmdletBinding()]
param([Parameter(Mandatory)][long]$WindowHandle)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$root = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$WindowHandle)
if ($null -eq $root) { throw 'Test-owned window has no UI Automation root.' }

function Find-NamedElement {
    param([string]$Name, [System.Windows.Automation.ControlType]$Type)
    $condition = [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty, $Name)
    if ($null -ne $Type) {
        $typeCondition = [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty, $Type)
        $condition = [System.Windows.Automation.AndCondition]::new($condition, $typeCondition)
    }
    $deadline = [DateTime]::UtcNow.AddSeconds(8)
    do {
        $element = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
        if ($null -ne $element) { return $element }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "UI Automation could not find '$Name'."
}

$terminal = Find-NamedElement -Name 'Terminal term1'
if (-not $terminal.Current.IsKeyboardFocusable) { throw 'Terminal node is not keyboard focusable.' }
$menu = Find-NamedElement -Name 'Terminal' -Type ([System.Windows.Automation.ControlType]::Button)
$menu.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
$readerAction = Find-NamedElement -Name ('Read terminal text' + [char]0x2026) -Type ([System.Windows.Automation.ControlType]::Button)
$readerAction.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
$reader = Find-NamedElement -Name 'Terminal output' -Type ([System.Windows.Automation.ControlType]::Edit)
$text = $reader.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern)
if ([string]::IsNullOrWhiteSpace($text.DocumentRange.GetText(200))) { throw 'Reader TextPattern did not expose terminal text.' }
$value = $reader.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
if (-not $value.Current.IsReadOnly) { throw 'Reader is not marked read-only in UI Automation.' }
$reader.SetFocus()
$text.DocumentRange.Select()
$selectionDeadline = [DateTime]::UtcNow.AddSeconds(5)
do {
    $selection = $text.GetSelection()
    if ($selection.Length -eq 1 -and -not [string]::IsNullOrWhiteSpace($selection[0].GetText(200))) { break }
    Start-Sleep -Milliseconds 100
} while ([DateTime]::UtcNow -lt $selectionDeadline)
if ($selection.Length -ne 1 -or [string]::IsNullOrWhiteSpace($selection[0].GetText(200))) { throw 'Reader text cannot be selected with UI Automation.' }
Write-Output 'Windows UI Automation: named/focusable terminal, menu Invoke, read-only reader TextPattern/ValuePattern and text selection pass.'
