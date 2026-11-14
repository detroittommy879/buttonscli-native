# ButtonsCLI installer for Windows (PowerShell 5.1 and 7).
#
#   irm https://buttonscli.com/install.ps1 | iex
#
# With options:
#   & ([scriptblock]::Create((irm https://buttonscli.com/install.ps1))) -Version 1.2.0
#   & ([scriptblock]::Create((irm https://buttonscli.com/install.ps1))) -Uninstall
# Environment: BUTTONSCLI_VERSION, BUTTONSCLI_INSTALL_DIR
#   (default %LOCALAPPDATA%\Programs\ButtonsCLI), BUTTONSCLI_NO_MODIFY_PATH=1,
#   BUTTONSCLI_UNINSTALL=1.
#
# Installs for your user only: no admin rights, no registry changes besides
# your user PATH. Settings live in %USERPROFILE%\.buttonscli-native.

param(
    [string]$Version = $env:BUTTONSCLI_VERSION,
    [string]$InstallDir = $env:BUTTONSCLI_INSTALL_DIR,
    [switch]$NoModifyPath,
    [switch]$Uninstall
)

function Install-ButtonsCLI {
    param([string]$Version, [string]$InstallDir, [bool]$NoModifyPath, [bool]$Uninstall)

    $ErrorActionPreference = 'Stop'
    # Invoke-WebRequest's progress bar makes downloads many times slower on 5.1.
    $ProgressPreference = 'SilentlyContinue'
    # Windows PowerShell 5.1 may default to TLS 1.0, which GitHub refuses.
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    } catch {}

    $repo = 'detroittommy879/buttonscli-native'
    if (-not $InstallDir) { $InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\ButtonsCLI' }
    $shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'ButtonsCLI.lnk'

    function Say([string]$text) { Write-Host "==> $text" -ForegroundColor Cyan }

    function Get-UserPath { [Environment]::GetEnvironmentVariable('Path', 'User') }
    function Test-OnUserPath([string]$dir) {
        $parts = (Get-UserPath) -split ';' | Where-Object { $_ } | ForEach-Object { $_.TrimEnd('\') }
        return $parts -contains $dir.TrimEnd('\')
    }

    if ($Uninstall) {
        Get-Process buttonscli -ErrorAction SilentlyContinue |
            Where-Object { $_.Path -and $_.Path.StartsWith($InstallDir, [StringComparison]::OrdinalIgnoreCase) } |
            ForEach-Object { throw "ButtonsCLI is running (PID $($_.Id)). Close it, then run the uninstaller again." }
        if (Test-Path $InstallDir) { Remove-Item -Recurse -Force $InstallDir; Say "Removed $InstallDir" }
        if (Test-Path $shortcut) { Remove-Item -Force $shortcut; Say 'Removed the Start Menu shortcut' }
        if (Test-OnUserPath $InstallDir) {
            $kept = (Get-UserPath) -split ';' | Where-Object { $_ -and $_.TrimEnd('\') -ne $InstallDir.TrimEnd('\') }
            [Environment]::SetEnvironmentVariable('Path', ($kept -join ';'), 'User')
            Say 'Removed it from your PATH'
        }
        Say 'ButtonsCLI removed. Settings are kept in %USERPROFILE%\.buttonscli-native.'
        return
    }

    if (-not [Environment]::Is64BitOperatingSystem) { throw 'ButtonsCLI needs 64-bit Windows.' }
    $cpu = $env:PROCESSOR_ARCHITEW6432
    if (-not $cpu) { $cpu = $env:PROCESSOR_ARCHITECTURE }
    if ($cpu -eq 'ARM64') {
        Write-Host 'ARM64 Windows: installing the x64 build (it runs under emulation).' -ForegroundColor Yellow
    }

    if (-not $Version) {
        # github.com/.../releases/latest redirects to .../tag/vX.Y.Z; no API
        # call, so no API rate limit. Fall back to the API if that changes.
        try {
            $response = Invoke-WebRequest -UseBasicParsing -Method Head "https://github.com/$repo/releases/latest"
            $final = if ($response.BaseResponse.ResponseUri) { $response.BaseResponse.ResponseUri.AbsoluteUri }
                     else { $response.BaseResponse.RequestMessage.RequestUri.AbsoluteUri }
            $tag = ($final -split '/')[-1]
        } catch { $tag = '' }
        if ($tag -notmatch '^v\d') {
            $tag = (Invoke-RestMethod -UseBasicParsing "https://api.github.com/repos/$repo/releases/latest").tag_name
        }
        if ($tag -notmatch '^v\d') { throw "Could not find the newest release (got '$tag'). Set BUTTONSCLI_VERSION and try again." }
        $Version = $tag
    }
    $Version = $Version.TrimStart('v')
    Say "Installing ButtonsCLI $Version"

    $asset = "buttonscli-$Version-windows-x64.zip"
    $base = "https://github.com/$repo/releases/download/v$Version"
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("buttonscli-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        Say "Downloading $asset"
        $zip = Join-Path $tmp $asset
        Invoke-WebRequest -UseBasicParsing "$base/$asset" -OutFile $zip
        Invoke-WebRequest -UseBasicParsing "$base/SHA256SUMS.txt" -OutFile (Join-Path $tmp 'SHA256SUMS.txt')
        $want = Get-Content (Join-Path $tmp 'SHA256SUMS.txt') |
            ForEach-Object { $parts = $_ -split '\s+', 2; if ($parts.Count -eq 2 -and $parts[1].TrimStart('*') -eq $asset) { $parts[0] } } |
            Select-Object -First 1
        if (-not $want) { throw "$asset is not listed in SHA256SUMS.txt." }
        $got = (Get-FileHash -Algorithm SHA256 $zip).Hash
        if ($got -ne $want) { throw "Checksum mismatch for $asset (expected $want, got $got). Nothing was installed." }
        Say 'Checksum OK'

        $unpacked = Join-Path $tmp 'unpacked'
        Expand-Archive -Path $zip -DestinationPath $unpacked -Force
        $src = Join-Path $unpacked "buttonscli-$Version-windows-x64"
        if (-not (Test-Path (Join-Path $src 'buttonscli.exe'))) { throw 'The archive has no buttonscli.exe.' }

        New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
        # Leftovers from the last update (they were in use then).
        Get-ChildItem $InstallDir -Filter '*.old' -ErrorAction SilentlyContinue |
            ForEach-Object { Remove-Item -Force $_.FullName -ErrorAction SilentlyContinue }
        foreach ($file in Get-ChildItem $src) {
            $dest = Join-Path $InstallDir $file.Name
            if (Test-Path $dest) {
                # A running exe can't be overwritten, but it can be renamed.
                $old = "$dest.$([Guid]::NewGuid().ToString('N').Substring(0, 8)).old"
                Move-Item -Force $dest $old
                Remove-Item -Force $old -ErrorAction SilentlyContinue
            }
            Copy-Item -Force $file.FullName $dest
        }
        # Files saved by a browser carry a "downloaded from the internet" mark;
        # make sure none is left on what we installed.
        Get-ChildItem $InstallDir -File | Unblock-File -ErrorAction SilentlyContinue
        $exe = Join-Path $InstallDir 'buttonscli.exe'
        Say "Installed $exe"

        $shell = New-Object -ComObject WScript.Shell
        $link = $shell.CreateShortcut($shortcut)
        $link.TargetPath = $exe
        $link.WorkingDirectory = $env:USERPROFILE
        $link.IconLocation = "$exe,0"
        $link.Description = 'ButtonsCLI terminal workspace'
        $link.Save()
        Say 'Added ButtonsCLI to the Start Menu'

        if (-not $NoModifyPath -and -not (Test-OnUserPath $InstallDir)) {
            $current = Get-UserPath
            $new = if ($current) { "$($current.TrimEnd(';'));$InstallDir" } else { $InstallDir }
            [Environment]::SetEnvironmentVariable('Path', $new, 'User')
            Say "Added $InstallDir to your PATH (open a new terminal to use it)"
        }
        if (-not (($env:Path -split ';') -contains $InstallDir)) { $env:Path = "$env:Path;$InstallDir" }

        Write-Host ''
        Say 'Done. Start ButtonsCLI from the Start Menu, or run: buttonscli'
        Write-Host '    Uninstall: & ([scriptblock]::Create((irm https://buttonscli.com/install.ps1))) -Uninstall'
    } finally {
        Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
    }
}

# Everything above only defines things, so a cut-off download runs nothing.
try {
    Install-ButtonsCLI -Version $Version -InstallDir $InstallDir `
        -NoModifyPath ([bool]$NoModifyPath -or $env:BUTTONSCLI_NO_MODIFY_PATH -eq '1') `
        -Uninstall ([bool]$Uninstall -or $env:BUTTONSCLI_UNINSTALL -eq '1')
} catch {
    Write-Host "error: $($_.Exception.Message)" -ForegroundColor Red
    # Under `irm | iex` a plain `exit` would close the user's window, so only
    # exit when run as a file; install.cmd reads LASTEXITCODE.
    $global:LASTEXITCODE = 1
    if ($MyInvocation.MyCommand.Path) { exit 1 }
}
