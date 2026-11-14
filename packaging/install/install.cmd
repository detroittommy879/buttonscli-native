@echo off
rem ButtonsCLI installer for Windows CMD.
rem
rem   curl -fsSL https://buttonscli.com/install.cmd -o install.cmd && install.cmd && del install.cmd
rem
rem Optional: install.cmd 1.2.0   (a specific version)
rem           install.cmd uninstall
rem It runs install.ps1 with Windows PowerShell, which unpacks the zip, checks
rem its SHA-256 and adds the Start Menu shortcut and PATH entry.
setlocal
set "BUTTONSCLI_PS1=https://buttonscli.com/install.ps1"
if /i "%~1"=="uninstall" (set "BUTTONSCLI_UNINSTALL=1") else if not "%~1"=="" set "BUTTONSCLI_VERSION=%~1"

where powershell >nul 2>nul
if errorlevel 1 (
  echo error: Windows PowerShell was not found. Download the zip from
  echo https://github.com/detroittommy879/buttonscli-native/releases/latest
  exit /b 1
)

powershell -NoProfile -ExecutionPolicy Bypass -Command "$ErrorActionPreference='Stop'; try { [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12 } catch {}; iex (irm -UseBasicParsing $env:BUTTONSCLI_PS1); exit $LASTEXITCODE"
exit /b %ERRORLEVEL%
