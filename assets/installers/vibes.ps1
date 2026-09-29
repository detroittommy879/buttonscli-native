#Requires -Version 5.1
#Requires -RunAsAdministrator
<#
.SYNOPSIS
Interactive rainbow menu installer for the dev environment components.
.DESCRIPTION
Presents a colorful menu that lets you choose which tools to install or refresh.
#>

param(
    [switch]$NoConfirm
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$host.UI.RawUI.WindowTitle = 'Vibe Code Dev Environment Installer 0.1'

$script:RainbowPalette = @('Red','Yellow','Green','Cyan','Blue','Magenta','DarkYellow','DarkCyan','DarkGreen','DarkMagenta')

function Disable-WindowsStorePythonStub {
    Write-Warn 'Windows Store Python stub detected. Attempting to disable it...'
    $appAliasesPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\App Paths'
    $stubPaths = @(
        "$env:LOCALAPPDATA\Microsoft\WindowsApps\python.exe",
        "$env:LOCALAPPDATA\Microsoft\WindowsApps\python3.exe"
    )
    
    foreach ($stubPath in $stubPaths) {
        if (Test-Path $stubPath) {
            try {
                Remove-Item $stubPath -Force -ErrorAction SilentlyContinue
                Write-Info "Removed stub: $stubPath"
            } catch {
                Write-Warn "Could not remove $stubPath - you may need to disable it in Settings > Apps > App execution aliases"
            }
        }
    }
    
    Write-Info 'Please also disable Python in: Settings > Apps > Advanced app settings > App execution aliases'
}

function Write-Info {
    param([string]$Message)
    Write-Host "-> $Message" -ForegroundColor Cyan
}

function Write-OK {
    param([string]$Message)
    Write-Host "OK: $Message" -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    Write-Host "!! $Message" -ForegroundColor Yellow
}

function Resolve-CommandPath {
    param([string]$Command)

    $extensions = @('cmd','exe','bat','ps1')
    foreach ($ext in $extensions) {
        try {
            $candidate = Get-Command ("{0}.{1}" -f $Command, $ext) -ErrorAction Stop
            $path = $null
            if ($candidate | Get-Member -Name Path -ErrorAction SilentlyContinue) { $path = $candidate.Path }
            if (-not $path -and ($candidate | Get-Member -Name Source -ErrorAction SilentlyContinue)) { $path = $candidate.Source }
            if ($path) { return $path }
        } catch {
        }
    }

    try {
        $candidate = Get-Command $Command -ErrorAction Stop
    } catch {
        return $null
    }

    $path = $null
    if ($candidate | Get-Member -Name Path -ErrorAction SilentlyContinue) { $path = $candidate.Path }
    if (-not $path -and ($candidate | Get-Member -Name Source -ErrorAction SilentlyContinue)) { $path = $candidate.Source }
    $path
}

function Get-CommandInfo {
    param([string]$Command)

    $path = Resolve-CommandPath $Command
    if (-not $path) {
        return $null
    }

    $normalized = $path.ToLowerInvariant()
    $isStoreStub = $false
    if ($normalized -like '*\\windowsapps\\*' -or $normalized -like '*appdata\\local\\microsoft\\windowsapps*' -or $normalized -like '*appdata\\local\\microsoft\\store*') {
        $isStoreStub = $true
    }

    [pscustomobject]@{
        Exists = $true
        Path = $path
        IsStoreStub = $isStoreStub
    }
}

function Test-Cmd {
    param(
        [string]$Command,
        [switch]$AllowStoreStub
    )

    $info = Get-CommandInfo $Command
    if (-not $info) {
        return $false
    }

    if ($info.IsStoreStub -and -not $AllowStoreStub) {
        return $false
    }

    $true
}

function Invoke-CommandChecked {
    param(
        [Parameter(Mandatory=$true)][string]$FilePath,
        [string[]]$Arguments = @(),
        [switch]$IgnoreExitCode
    )

    & $FilePath @Arguments

    if ($IgnoreExitCode) { return }

    $exit = $LASTEXITCODE
    if ($null -ne $exit -and $exit -ne 0) {
        throw "Command '$FilePath' exited with code $exit."
    }
}

function Invoke-Tool {
    param(
        [Parameter(Mandatory=$true)][string]$Command,
        [string[]]$Arguments = @()
    )

    $path = Resolve-CommandPath $Command
    if (-not $path) {
        throw "Command '$Command' not found on PATH."
    }

    Invoke-CommandChecked -FilePath $path -Arguments $Arguments
}

function Resolve-PythonExe {
    $info = Get-CommandInfo 'python'
    if ($info -and -not $info.IsStoreStub -and $info.Path -and (Test-Path $info.Path)) {
        # Double-check it's not the WindowsApps stub even if IsStoreStub didn't catch it
        if ($info.Path -notlike '*\WindowsApps\*' -and $info.Path -notlike '*\Microsoft\WindowsApps\*') {
            return $info.Path
        }
    }

    $candidates = @()
    $candidates += 'C:\Python312\python.exe'
    $candidates += 'C:\Python311\python.exe'
    $candidates += 'C:\Python310\python.exe'
    $candidates += 'C:\ProgramData\chocolatey\bin\python.exe'
    $candidates += 'C:\ProgramData\chocolatey\lib\python\tools\python.exe'
    $candidates += 'C:\ProgramData\chocolatey\lib\python312\tools\python.exe'
    $candidates += 'C:\ProgramData\chocolatey\lib\python311\tools\python.exe'
    if ($env:ProgramFiles) {
        $candidates += (Join-Path $env:ProgramFiles 'Python312\python.exe')
        $candidates += (Join-Path $env:ProgramFiles 'Python311\python.exe')
        $candidates += (Join-Path $env:ProgramFiles 'Python310\python.exe')
    }
    if ($env:LOCALAPPDATA) {
        $candidates += (Join-Path $env:LOCALAPPDATA 'Programs\Python\Python312\python.exe')
        $candidates += (Join-Path $env:LOCALAPPDATA 'Programs\Python\Python311\python.exe')
        $candidates += (Join-Path $env:LOCALAPPDATA 'Programs\Python\Python310\python.exe')
    }

    foreach ($candidate in $candidates | Where-Object { $_ }) {
        if (Test-Path $candidate) {
            # Verify it's a real Python by trying to get version
            try {
                $testOutput = & $candidate --version 2>&1
                if ($testOutput -match 'Python \d+\.\d+') {
                    return $candidate
                }
            } catch {
                # Skip this candidate if it fails
                continue
            }
        }
    }

    try {
        $chocoBin = Join-Path $env:ProgramData 'chocolatey\bin'
        if (Test-Path $chocoBin) {
            $pythonShim = Get-ChildItem -Path $chocoBin -Filter 'python*.exe' -ErrorAction SilentlyContinue | Sort-Object Name | Select-Object -First 1
            if ($pythonShim) { return $pythonShim.FullName }
        }
    } catch {}

    $registryRoots = @(
        'HKEY_LOCAL_MACHINE\SOFTWARE\Python\PythonCore\3.12\InstallPath',
        'HKEY_LOCAL_MACHINE\SOFTWARE\Python\PythonCore\3.11\InstallPath',
        'HKEY_LOCAL_MACHINE\SOFTWARE\Python\PythonCore\3.12-32\InstallPath'
    )

    foreach ($root in $registryRoots) {
        try {
            $installPath = [Microsoft.Win32.Registry]::GetValue($root, '', $null)
            if ($installPath) {
                $candidate = Join-Path $installPath 'python.exe'
                if (Test-Path $candidate) {
                    return $candidate
                }
            }
        } catch {
        }
    }

    if ($info -and -not $info.IsStoreStub -and $info.Path) {
        if (Test-Path $info.Path) {
            return $info.Path
        }
    }

    $null
}

function Refresh-EnvPath {
    if (Get-Command refreshenv -ErrorAction SilentlyContinue) {
        refreshenv | Out-Null
    }
    $machinePath = [System.Environment]::GetEnvironmentVariable('PATH', 'Machine')
    $userPath = [System.Environment]::GetEnvironmentVariable('PATH', 'User')
    $env:PATH = "$machinePath$([IO.Path]::PathSeparator)$userPath"
}

function Install-Chocolatey {
    if (Test-Cmd 'choco') {
        Write-OK 'Chocolatey already present'
        return
    }

    Write-Info 'Installing Chocolatey...'
    Set-ExecutionPolicy Bypass -Scope Process -Force
    [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.SecurityProtocolType]::Tls12
    Invoke-Expression ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
    Refresh-EnvPath
    Import-Module "$env:ChocolateyInstall\helpers\chocolateyProfile.psm1" -Force -ErrorAction SilentlyContinue
    Write-OK 'Chocolatey ready'
}

function Install-Git {
    if (Test-Cmd 'git') {
        Write-OK 'Git already present'
        return
    }

    Install-Chocolatey
    Write-Info 'Installing Git...'
    Invoke-Tool -Command 'choco' -Arguments @('install','git','-y','--no-progress')
    Refresh-EnvPath
    Write-OK 'Git installed'
}

function Install-NodeLts {
    if (Test-Cmd 'node') {
        Write-OK 'Node.js already present'
        return
    }

    Install-Chocolatey
    Write-Info 'Installing Node.js LTS...'
    Invoke-Tool -Command 'choco' -Arguments @('install','nodejs-lts','-y','--no-progress')
    Refresh-EnvPath
    Write-OK 'Node.js installed'
    npm install -g npm@latest --quiet 2>$null
}

function Install-PythonStack {
    # Aggressive Windows Store stub removal
    $stubPaths = @(
        "$env:LOCALAPPDATA\Microsoft\WindowsApps\python.exe",
        "$env:LOCALAPPDATA\Microsoft\WindowsApps\python3.exe",
        "$env:LOCALAPPDATA\Microsoft\WindowsApps\python.bat",
        "$env:LOCALAPPDATA\Microsoft\WindowsApps\python3.bat"
    )
    
    foreach ($stubPath in $stubPaths) {
        if (Test-Path $stubPath) {
            try {
                Remove-Item $stubPath -Force -ErrorAction SilentlyContinue
                Write-Host "Removed stub: $stubPath" -ForegroundColor Cyan
            } catch {
                Write-Host "Warning: Could not remove $stubPath" -ForegroundColor Yellow
            }
        }
    }

    $pythonExe = Resolve-PythonExe
    $pythonPresent = $pythonExe -ne $null

    if (-not $pythonPresent) {
        Install-Chocolatey
        Write-Info 'Installing Python 3.12 via Chocolatey...'
        
        # Use --force to ensure installation even if partial install exists
        Invoke-Tool -Command 'choco' -Arguments @('install','python','--version=3.12','-y','--no-progress','--force')
        
        # Refresh environment thoroughly
        $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH","User")
        
        # Wait and retry multiple times for Python to appear
        $retryCount = 0
        $maxRetries = 10
        while (-not $pythonExe -and $retryCount -lt $maxRetries) {
            Start-Sleep -Seconds 2
            $pythonExe = Resolve-PythonExe
            $retryCount++
            if (-not $pythonExe) {
                Write-Host "Waiting for Python... ($retryCount/$maxRetries)" -ForegroundColor Yellow
            }
        }
        
        if (-not $pythonExe) {
            # Check Chocolatey installation directory directly
            $chocoPythonPath = 'C:\ProgramData\chocolatey\lib\python312\tools\python.exe'
            if (Test-Path $chocoPythonPath) {
                $pythonExe = $chocoPythonPath
                Write-Host "Using direct Chocolatey path: $pythonExe" -ForegroundColor Cyan
            }
        }
        
        if (-not $pythonExe) {
            throw 'Python installation completed but python executable not found. Check C:\ProgramData\chocolatey\lib\python312\tools\ manually.'
        }
        Write-OK 'Python installed'
        & $pythonExe -m pip install --upgrade pip --quiet
    } else {
        Write-OK 'Python already present'
    }

    Write-Info "Using python executable: $pythonExe"
    
    # Ensure we can actually run Python
    try {
        $pythonVersion = & $pythonExe --version 2>&1
        Write-Host "Python version: $pythonVersion" -ForegroundColor Green
    } catch {
        throw "Cannot execute Python at $pythonExe - $($_.Exception.Message)"
    }

    # Install/upgrade pip
    Write-Info 'Ensuring pip is available...'
    try {
        Invoke-CommandChecked -FilePath $pythonExe -Arguments @('-m','ensurepip','--upgrade')
    } catch {
        Write-Warn "ensurepip reported: $($_.Exception.Message)"
    }
    
    # Refresh PATH again
    $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH","User")
    
    # Install pipx and uv
    Write-Info 'Installing pipx...'
    Invoke-CommandChecked -FilePath $pythonExe -Arguments @('-m','pip','install','--user','--upgrade','pipx')
    Invoke-CommandChecked -FilePath $pythonExe -Arguments @('-m','pipx','ensurepath')
    
    Write-Info 'Installing uv...'
    Invoke-CommandChecked -FilePath $pythonExe -Arguments @('-m','pip','install','--user','--upgrade','uv')
    
    # Final PATH refresh
    $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH","User")
    Write-OK 'Python stack (pip, pipx, uv) installed'
}

function Install-AICodePrep {
    Install-PythonStack
    if (Test-Cmd 'aicodeprep-gui') {
        Write-OK 'aicodeprep-gui already present'
        return
    }

    $pythonExe = Resolve-PythonExe
    if (-not $pythonExe) {
        throw 'Unable to resolve python executable for pipx.'
    }

    Write-Info 'Installing aicodeprep-gui via pipx...'
    Invoke-CommandChecked -FilePath $pythonExe -Arguments @('-m','pipx','install','aicodeprep-gui')
    Invoke-CommandChecked -FilePath $pythonExe -Arguments @('-m','pipx','ensurepath')
    Refresh-EnvPath
    Write-OK 'aicodeprep-gui installed'
}

function Install-VSCode {
    if (Test-Cmd 'code') {
        Write-OK 'VS Code already present'
        return
    }

    Install-Chocolatey
    Write-Info 'Installing VS Code (user setup)...'
    Invoke-Tool -Command 'choco' -Arguments @('install','vscode','-y','--no-progress')
    $vscodeBin = Join-Path $env:LOCALAPPDATA 'Programs\Microsoft VS Code\bin'
    if (Test-Path $vscodeBin) {
        $env:PATH = "$vscodeBin;$($env:PATH)"
    }
    Refresh-EnvPath
    Write-OK 'VS Code installed'
}

function Install-VSCodeExtensions {
    Install-VSCode
    if (-not (Test-Cmd 'code')) {
        throw 'VS Code command line not available.'
    }

    $extensions = @(
        'saoudrizwan.claude-dev',
        'kilocode.Kilo-Code',
        'ms-vscode.live-server',
        'oderwat.indent-rainbow',
        'esbenp.prettier-vscode'
    )

    foreach ($ext in $extensions) {
        Write-Info "Installing VS Code extension $ext..."
        Invoke-Tool -Command 'code' -Arguments @('--install-extension',$ext,'--force')
    }
    Write-OK 'VS Code extensions installed'
}

function Install-QwenCode {
    Install-NodeLts
    if (-not (Test-Cmd 'npm')) {
        throw 'npm command not available; Node.js PATH update has not taken effect yet.'
    }
    if (Test-Cmd 'qwen') {
        Write-OK '@qwen-code/qwen-code already present'
        return
    }

    Write-Info 'Installing @qwen-code/qwen-code...'
    Invoke-Tool -Command 'npm' -Arguments @('install','-g','@qwen-code/qwen-code@latest')
    Refresh-EnvPath
    Write-OK '@qwen-code/qwen-code installed'
}

function Install-GeminiCli {
    Install-NodeLts
    if (-not (Test-Cmd 'npm')) {
        throw 'npm command not available; Node.js PATH update has not taken effect yet.'
    }
    if (Test-Cmd 'gemini') {
        Write-OK '@google/gemini-cli already present'
        return
    }

    Write-Info 'Installing @google/gemini-cli...'
    Invoke-Tool -Command 'npm' -Arguments @('install','-g','@google/gemini-cli')
    Refresh-EnvPath
    Write-OK '@google/gemini-cli installed'
}

function Install-CodexCli {
    Install-NodeLts
    if (-not (Test-Cmd 'npm')) {
        throw 'npm command not available; Node.js PATH update has not taken effect yet.'
    }
    if (Test-Cmd 'codex') {
        Write-OK '@openai/codex already present'
        return
    }

    Write-Info 'Installing @openai/codex...'
    Invoke-Tool -Command 'npm' -Arguments @('install','-g','@openai/codex')
    Refresh-EnvPath
    Write-OK '@openai/codex installed'
}

function Install-ClaudeCodeCli {
    Install-NodeLts
    if (-not (Test-Cmd 'npm')) {
        throw 'npm command not available; Node.js PATH update has not taken effect yet.'
    }
    if ((Test-Cmd 'claude') -or (Test-Cmd 'claude-code')) {
        Write-OK '@anthropic-ai/claude-code already present'
        return
    }

    Write-Info 'Installing @anthropic-ai/claude-code...'
    Invoke-Tool -Command 'npm' -Arguments @('install','-g','@anthropic-ai/claude-code')
    Refresh-EnvPath
    Write-OK '@anthropic-ai/claude-code installed'
}

function Get-RainbowColor {
    param([int]$Index)
    if ($script:RainbowPalette.Count -eq 0) {
        return 'White'
    }
    $paletteIndex = $Index % $script:RainbowPalette.Count
    $script:RainbowPalette[$paletteIndex]
}

function Format-CellText {
    param(
        $Task,
        [int]$Width
    )

    if ($Width -le 0) {
        return "$($Task.Name)"
    }

    $mark = if ($Task.Selected) { '[x]' } else { '[ ]' }
    $status = if ($Task.Installed) { 'Possibly already installed' } else { 'Available' }
    if ($Task.Detail) {
        $status = "$status | $($Task.Detail)"
    }
    $text = "$mark $($Task.Name) - $status"

    if ($text.Length -gt $Width) {
        if ($Width -gt 4) {
            $text = $text.Substring(0, $Width - 3) + '...'
        } elseif ($Width -gt 0) {
            $text = $text.Substring(0, $Width)
        }
    }

    if ($Width -gt 0 -and $text.Length -lt $Width) {
        $text = $text.PadRight($Width)
    }

    $text
}

function New-InstallerTask {
    param(
        [string]$Id,
        [string]$Name,
        [string]$Description,
        [ScriptBlock]$Detect,
        [ScriptBlock]$Install
    )

    [pscustomobject]@{
        Id = $Id
        Name = $Name
        Description = $Description
        Detect = $Detect
        Install = $Install
        Selected = $false
        Installed = $false
        Detail = ''
        Initialized = $false
    }
}

function Get-Layout {
    param($Tasks)

    $size = $host.UI.RawUI.WindowSize
    $columns = 1
    $rows = $Tasks.Count
    $columnWidth = [Math]::Max(50, $size.Width - 4)

    $grid = @()
    $positions = @{}

    for ($row = 0; $row -lt $rows; $row++) {
        $gridRow = @($row)
        $grid += ,$gridRow
        $positions[$row] = [pscustomobject]@{ Row = $row; Col = 0 }
    }

    [pscustomobject]@{
        Columns = $columns
        Rows = $rows
        ColumnWidth = $columnWidth
        Grid = $grid
        Positions = $positions
        Size = $size
    }
}

function Get-NextIndex {
    param(
        [int]$Current,
        [string]$Direction,
        $Layout
    )

    $totalItems = $Layout.Rows

    switch ($Direction) {
        'Up' {
            if ($Current -le 0) {
                return $totalItems - 1
            } else {
                return $Current - 1
            }
        }
        'Down' {
            if ($Current -ge ($totalItems - 1)) {
                return 0
            } else {
                return $Current + 1
            }
        }
        default {
            return $Current
        }
    }
}

function Render-Menu {
    param(
        $Tasks,
        [int]$CurrentIndex,
        $Layout
    )

    Clear-Host

    $title = 'Code with AI Helper / Vibe Coding Installer'
    for ($i = 0; $i -lt $title.Length; $i++) {
        $color = Get-RainbowColor $i
        Write-Host $title[$i] -ForegroundColor $color -NoNewline
    }
    Write-Host ''
    Write-Host ('-' * [Math]::Min($Layout.Size.Width, 80)) -ForegroundColor DarkGray

    for ($row = 0; $row -lt $Layout.Rows; $row++) {
        for ($col = 0; $col -lt $Layout.Columns; $col++) {
            $index = ($Layout.Grid[$row])[$col]
            if ($null -eq $index) {
                continue
            }

            $task = $Tasks[$index]
            $text = Format-CellText -Task $task -Width $Layout.ColumnWidth
            $isCurrent = $index -eq $CurrentIndex
            $fg = if ($isCurrent) { 'Black' } else { Get-RainbowColor $index }
            $bg = if ($isCurrent) { 'Gray' } else { 'Black' }

            Write-Host $text -ForegroundColor $fg -BackgroundColor $bg -NoNewline
            if ($col -lt ($Layout.Columns - 1)) {
                Write-Host (' ' * 4) -NoNewline
            }
        }
        Write-Host ''
    }

    Write-Host ''
    $current = $Tasks[$CurrentIndex]
    Write-Host ("Selected: {0}" -f $current.Name) -ForegroundColor White
    Write-Host $current.Description -ForegroundColor Gray
    if ($current.Detail) {
        Write-Host ("Status: {0}" -f $current.Detail) -ForegroundColor DarkGray
    }
    Write-Host ''
    Write-Host 'Use arrows to move, Space to toggle, Enter to install, R to refresh, Esc to quit.' -ForegroundColor DarkGray
    Write-Host ''
}

function Update-TaskDetection {
    param($Task)

    try {
        $state = & $Task.Detect
    } catch {
        $state = [pscustomobject]@{ Installed = $false; Details = $_.Exception.Message }
    }

    $Task.Installed = [bool]$state.Installed
    $Task.Detail = $state.Details

    if (-not $Task.Initialized) {
        $Task.Selected = $true
        $Task.Initialized = $true
    }

    $Task
}

function Get-TaskDefinitions {
    @(
        (New-InstallerTask -Id 'choco' -Name 'Chocolatey' -Description 'Package manager used to bootstrap the rest.' -Detect {
                $installed = Test-Cmd 'choco'
                [pscustomobject]@{
                    Installed = $installed
                    Details = ''
                }
            } -Install { Install-Chocolatey })
        (New-InstallerTask -Id 'git' -Name 'Git for Windows' -Description 'Source control client.' -Detect {
                $installed = Test-Cmd 'git'
                [pscustomobject]@{ Installed = $installed; Details = '' }
            } -Install { Install-Git })
        (New-InstallerTask -Id 'node' -Name 'Node.js LTS + npm' -Description 'JavaScript runtime and package manager.' -Detect {
                $installed = Test-Cmd 'node'
                [pscustomobject]@{ Installed = $installed; Details = '' }
            } -Install { Install-NodeLts })
        (New-InstallerTask -Id 'python' -Name 'Python 3.12 + pip/pipx/uv' -Description 'Python runtime and supporting package tools.' -Detect {
                $pythonExe = Resolve-PythonExe
                if (-not $pythonExe) {
                    return [pscustomobject]@{ Installed = $false; Details = '' }
                }
                $pip = Test-Cmd 'pip'
                $pipx = Test-Cmd 'pipx'
                $uv = Test-Cmd 'uv'

                $scriptDirs = @()
                $pythonDir = Split-Path $pythonExe
                if ($pythonDir) { $scriptDirs += (Join-Path $pythonDir 'Scripts') }
                if ($env:APPDATA) { $scriptDirs += (Join-Path $env:APPDATA 'Python\Python312\Scripts') }
                if ($env:LOCALAPPDATA) { $scriptDirs += (Join-Path $env:LOCALAPPDATA 'Programs\Python\Python312\Scripts') }

                if (-not $pip) {
                    foreach ($dir in $scriptDirs) {
                        if ($dir -and (Test-Path (Join-Path $dir 'pip.exe'))) { $pip = $true; break }
                        if ($dir -and (Test-Path (Join-Path $dir 'pip3.exe'))) { $pip = $true; break }
                    }
                }

                if (-not $pipx) {
                    foreach ($dir in $scriptDirs) {
                        if ($dir -and (Test-Path (Join-Path $dir 'pipx.exe'))) { $pipx = $true; break }
                    }
                }

                if (-not $uv) {
                    foreach ($dir in $scriptDirs) {
                        if ($dir -and (Test-Path (Join-Path $dir 'uv.exe'))) { $uv = $true; break }
                    }
                }
                $all = $pip -and $pipx -and $uv
                [pscustomobject]@{ Installed = $all; Details = '' }
            } -Install { Install-PythonStack })
        (New-InstallerTask -Id 'aicodeprep' -Name 'aicodeprep-gui (pipx)' -Description 'AI Code Prep GUI CLI via pipx.' -Detect {
                $installed = Test-Cmd 'aicodeprep-gui'
                [pscustomobject]@{
                    Installed = $installed
                    Details = ''
                }
            } -Install { Install-AICodePrep })
        (New-InstallerTask -Id 'vscode' -Name 'Visual Studio Code' -Description 'Primary code editor.' -Detect {
                $installed = Test-Cmd 'code'
                [pscustomobject]@{ Installed = $installed; Details = '' }
            } -Install { Install-VSCode })
        (New-InstallerTask -Id 'extensions' -Name 'VS Code Extensions (5 total)' -Description 'Claude Dev, Kilo-Code, Live Server, Indent Rainbow, Prettier.' -Detect {
                if (-not (Test-Cmd 'code')) {
                    return [pscustomobject]@{ Installed = $false; Details = '' }
                }
                try {
                    $extensions = code --list-extensions
                    $required = @('saoudrizwan.claude-dev', 'kilocode.Kilo-Code', 'ms-vscode.live-server', 'oderwat.indent-rainbow', 'esbenp.prettier-vscode')
                    $installed = $required | Where-Object { $_ -in $extensions }
                    $count = $installed.Count
                    [pscustomobject]@{ Installed = ($count -eq $required.Count); Details = "$($count)/$($required.Count) installed" }
                } catch {
                    [pscustomobject]@{ Installed = $false; Details = '' }
                }
            } -Install { Install-VSCodeExtensions })
        (New-InstallerTask -Id 'qwen' -Name 'Global npm: @qwen-code/qwen-code' -Description 'Qwen code assistant CLI.' -Detect {
                $installed = Test-Cmd 'qwen'
                [pscustomobject]@{
                    Installed = $installed
                    Details = ''
                }
            } -Install { Install-QwenCode })
        (New-InstallerTask -Id 'gemini' -Name 'Global npm: @google/gemini-cli' -Description 'Gemini CLI from Google.' -Detect {
                $installed = Test-Cmd 'gemini'
                [pscustomobject]@{
                    Installed = $installed
                    Details = ''
                }
            } -Install { Install-GeminiCli })
        (New-InstallerTask -Id 'codex' -Name 'OpenAI Codex CLI' -Description 'OpenAI Codex CLI for AI-assisted coding.' -Detect {
                $installed = Test-Cmd 'codex'
                [pscustomobject]@{
                    Installed = $installed
                    Details = ''
                }
            } -Install { Install-CodexCli })
        (New-InstallerTask -Id 'claude' -Name 'Claude Code CLI' -Description 'Claude Code from Anthropic for AI-assisted development.' -Detect {
                $commandName = if (Test-Cmd 'claude') { 'claude' } elseif (Test-Cmd 'claude-code') { 'claude-code' } else { $null }
                [pscustomobject]@{
                    Installed = $null -ne $commandName
                    Details = if ($commandName) { "CLI detected ($commandName)" } else { '' }
                }
            } -Install { Install-ClaudeCodeCli })
    )
}

$tasks = Get-TaskDefinitions
foreach ($task in $tasks) {
    Update-TaskDetection $task | Out-Null
}

$currentIndex = 0
$menuActive = $true
$confirmed = $false

while ($menuActive) {
    $layout = Get-Layout $tasks
    Render-Menu -Tasks $tasks -CurrentIndex $currentIndex -Layout $layout
    $key = [Console]::ReadKey($true)

    switch ($key.Key) {
        'UpArrow' { $currentIndex = Get-NextIndex -Current $currentIndex -Direction 'Up' -Layout $layout }
        'DownArrow' { $currentIndex = Get-NextIndex -Current $currentIndex -Direction 'Down' -Layout $layout }
        'Spacebar' {
            $tasks[$currentIndex].Selected = -not $tasks[$currentIndex].Selected
        }
        'R' {
            foreach ($task in $tasks) {
                Update-TaskDetection $task | Out-Null
            }
        }
        'Enter' {
            $confirmed = $true
            $menuActive = $false
        }
        'Escape' {
            $menuActive = $false
        }
        default {
            if ($key.KeyChar -eq 'r' -or $key.KeyChar -eq 'R') {
                foreach ($task in $tasks) {
                    Update-TaskDetection $task | Out-Null
                }
            }
        }
    }
}

if (-not $confirmed) {
    Write-Warn 'Installation cancelled.'
    return
}

$selectedTasks = $tasks | Where-Object { $_.Selected }
if (-not $selectedTasks) {
    Write-Warn 'No components selected. Nothing to install.'
    return
}

if (-not $NoConfirm) {
    Write-Host ''
    Write-Host 'Selected components:' -ForegroundColor Cyan
    foreach ($task in $selectedTasks) {
        Write-Host " - $($task.Name)" -ForegroundColor Gray
    }
    $response = Read-Host 'Press Enter to continue or type N to cancel'
    if ($response -match '^[Nn]') {
        Write-Warn 'Installation aborted.'
        return
    }
}

Clear-Host
Write-Host 'Starting installation...' -ForegroundColor Cyan

foreach ($task in $selectedTasks) {
    Write-Host ''
    Write-Info "Processing $($task.Name)..."
    try {
        & $task.Install
        Update-TaskDetection $task | Out-Null
        Write-OK "$($task.Name) complete"
    } catch {
        Write-Host "!! Failed to install $($task.Name): $($_.Exception.Message)" -ForegroundColor Red
    }
}
# Final robust re-check: sometimes npm/pipx updates PATH for current session slowly
# Use cmd.exe's 'where' and Resolve-PythonExe as a fallback to ensure summary is accurate
foreach ($task in $tasks) {
    try {
        switch ($task.Id) {
            'python' {
                $py = Resolve-PythonExe
                if ($py) { $task.Installed = $true; $task.Detail = $py }
                else { $task.Installed = $false; $task.Detail = '' }
            }
            'extensions' {
                if (Test-Cmd 'code') {
                    try {
                        $extensions = code --list-extensions 2>$null
                        $required = @('saoudrizwan.claude-dev', 'kilocode.Kilo-Code', 'ms-vscode.live-server', 'oderwat.indent-rainbow', 'esbenp.prettier-vscode')
                        $installed = $required | Where-Object { $_ -in $extensions }
                        $count = $installed.Count
                        $task.Installed = ($count -eq $required.Count)
                        $task.Detail = "$($count)/$($required.Count) installed"
                    } catch {
                        $task.Installed = $false; $task.Detail = ''
                    }
                } else {
                    $task.Installed = $false; $task.Detail = ''
                }
            }
            'claude' {
                $out = & 'cmd.exe' '/c' 'where claude' 2>&1
                if ($LASTEXITCODE -ne 0 -or -not $out -or ($out -match 'Could not find')) {
                    $out = & 'cmd.exe' '/c' 'where claude-code' 2>&1
                }
                if ($LASTEXITCODE -eq 0 -and $out -and ($out -notmatch 'Could not find')) {
                    $task.Installed = $true
                    $task.Detail = ($out -join ';').Trim()
                } else {
                    $task.Installed = $false
                    if (-not $task.Detail) { $task.Detail = '' }
                }
            }
            default {
                $cmdNames = @{
                    choco = 'choco'; git = 'git'; node = 'node'; aicodeprep = 'aicodeprep-gui';
                    vscode = 'code'; qwen = 'qwen'; gemini = 'gemini'; codex = 'codex'
                }
                if ($cmdNames.ContainsKey($task.Id)) {
                    $cmdName = $cmdNames[$task.Id]
                    try {
                        $out = & 'cmd.exe' '/c' "where $cmdName" 2>&1
                        if ($LASTEXITCODE -eq 0 -and $out -and ($out -notmatch 'Could not find')) {
                            $task.Installed = $true
                            $task.Detail = ($out -join ';').Trim()
                        } else {
                            $task.Installed = $false
                            # preserve any existing detail if it contains useful info
                            if (-not $task.Detail) { $task.Detail = '' }
                        }
                    } catch {
                        # If where fails, fall back to the existing detection state
                    }
                }
            }
        }
    } catch {
        # keep whatever state is already present on unexpected errors
    }
}

Write-Host ''
Write-Host 'Summary:' -ForegroundColor Cyan
foreach ($task in $tasks) {
    $status = if ($task.Installed) { 'Installed' } else { 'Missing' }
    $detailSuffix = if ($task.Detail) { " - $($task.Detail)" } else { '' }
    $color = if ($task.Installed) { 'Green' } else { 'Yellow' }
    Write-Host ("[{0}] {1}{2}" -f $status, $task.Name, $detailSuffix) -ForegroundColor $color
}

Write-Host ''
Write-Host 'All done! Enjoy your new dev box.' -ForegroundColor Magenta
Write-Host 'Tip: open a new non-admin terminal session so PATH updates are visible.' -ForegroundColor DarkGray
Write-Host ''

# Create clickable hyperlink using ANSI escape sequences (works in Windows Terminal & VS Code)
$url = 'https://wuu73.org/blog'
$linkText = 'Click here for useful links and information about cheap/free AI coding methods or tools'
$e = [char]27
$hyperlink = "$e]8;;$url$e\$linkText$e]8;;$e\"
Write-Host $hyperlink -ForegroundColor Cyan

# Also show plain URL for terminals that don't support hyperlinks
Write-Host "Or visit: $url" -ForegroundColor Gray

# Offer to open in browser
Write-Host ''
$response = Read-Host "Open the blog in your browser now? (Y/n)"
if ($response -notmatch '^[Nn]') {
    try {
        Start-Process $url
        Write-Host 'Opening browser...' -ForegroundColor Green
    } catch {
        Write-Host "Could not open browser automatically. Please visit: $url" -ForegroundColor Yellow
    }
}
# ------------------------------------------------------------------
# Desktop shortcut that stays open so the user can read the result
# ------------------------------------------------------------------
$desktop   = [Environment]::GetFolderPath('Desktop')
$ps1File   = Join-Path $desktop 'Fix-PowerShell-for-CLI.ps1'
$lnkFile   = Join-Path $desktop 'Fix PowerShell for CLI.lnk'

# script that fixes policy and waits for a key
@'
Write-Host "`nSetting execution policy to RemoteSigned (current-user scope)..."
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
Write-Host "`nAll done! You can now run CLI tools from PowerShell." -ForegroundColor Green
Write-Host "`nPress any key to close this window..."
$null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
'@ | Out-File -FilePath $ps1File -Encoding ASCII -Force
Unblock-File -Path $ps1File -ErrorAction SilentlyContinue

# create the shortcut
$shell  = New-Object -ComObject WScript.Shell
$lnk    = $shell.CreateShortcut($lnkFile)
$lnk.TargetPath        = 'powershell.exe'
$lnk.Arguments         = "-NoProfile -ExecutionPolicy Bypass -NoExit -File `"$ps1File`""
$lnk.WorkingDirectory  = $desktop
$lnk.WindowStyle       = 1   # normal window
$lnk.Description       = 'Allow PowerShell to run CLI tools (RemoteSigned)'
$lnk.Save()

$quickStartFile = Join-Path $desktop 'Vibe-Coding-Quick-Start.txt'
@'
Vibe Coding Quick Start

Open a NEW terminal window after the installer finishes so PATH updates are visible.

1. Qwen Code
   qwen
   Follow the browser sign-in flow, then type /quit when you are done.

2. Gemini CLI
   gemini
   Follow the browser sign-in flow, then type /quit when you are done.

3. VS Code + AI extensions
   Open Visual Studio Code from the Start menu.
   Open Cline or Kilo Code from the left sidebar.
   Choose Qwen / Qwen Code as the provider if you want a free starting point.

Other useful commands:
   aicp
   codex
   claude

If PowerShell says scripts are disabled, double-click:
   Fix PowerShell for CLI.lnk
'@ | Out-File -FilePath $quickStartFile -Encoding ASCII -Force

Write-Host ''
Write-Host 'Created desktop shortcut:  Fix PowerShell for CLI.lnk' -ForegroundColor Green
Write-Host 'Double-click it any time a CLI complains about scripts being disabled.' -ForegroundColor Gray
Write-Host "Created desktop quick-start guide: $quickStartFile" -ForegroundColor Green
