[CmdletBinding()]
param(
    # Terminals to compare. Missing ones are skipped with a note.
    [string[]]$Terminals = @('buttonscli', 'alacritty', 'wezterm', 'windows-terminal', 'conhost'),
    [string[]]$Workloads = @('scrolling', 'dense_cells', 'unicode', 'truecolor', 'cursor_motion'),
    [ValidateRange(1, 256)]
    [int]$MiB = 16,
    [ValidateRange(1, 20)]
    [int]$Runs = 3,
    [ValidateRange(10, 600)]
    [int]$TimeoutSeconds = 120,
    [string]$ButtonsCliPath,
    [switch]$SkipBuild,
    [string]$OutputPath
)

# Throughput, startup and memory comparison against other terminals.
# Every terminal runs the same writer (`term_bench --cat`) with the same bytes;
# the writer's stdout is that terminal's PTY, so the write only finishes as
# fast as the terminal consumes it. ButtonsCLI runs in an isolated profile.
#
#   .\scripts\bench-terminals.ps1
#   .\scripts\bench-terminals.ps1 -Terminals buttonscli,alacritty -MiB 32 -Runs 5
#
# Input latency is not measured here; see docs/BENCHMARKS.md (Typometer).

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$workDir = Join-Path ([System.IO.Path]::GetTempPath()) "buttonscli-bench-$([guid]::NewGuid().ToString('N'))"
$workloadDir = Join-Path $workDir 'workloads'
$isolatedHome = Join-Path $workDir 'home'
New-Item -ItemType Directory -Path $workloadDir, (Join-Path $isolatedHome 'AppData\Roaming'), (Join-Path $isolatedHome 'AppData\Local') -Force | Out-Null

Push-Location $repoRoot
try {
    if (-not $SkipBuild) {
        & cargo build --release --locked -p egui_term --example term_bench
        if ($LASTEXITCODE -ne 0) { throw 'building term_bench failed' }
        if (-not $ButtonsCliPath) {
            & cargo build --release --locked --bin buttonscli
            if ($LASTEXITCODE -ne 0) { throw 'building buttonscli failed' }
        }
    }
} finally {
    Pop-Location
}

$writer = Join-Path $repoRoot 'target\release\examples\term_bench.exe'
if (-not (Test-Path -LiteralPath $writer)) { throw "Writer not found: $writer" }
if (-not $ButtonsCliPath) { $ButtonsCliPath = Join-Path $repoRoot 'target\release\buttonscli.exe' }

& $writer --emit $workloadDir --mib $MiB | Out-Null

function Find-Exe([string[]]$Candidates) {
    foreach ($candidate in $Candidates) {
        if ([string]::IsNullOrWhiteSpace($candidate)) { continue }
        $command = Get-Command $candidate -ErrorAction SilentlyContinue
        if ($command) { return $command.Source }
        if (Test-Path -LiteralPath $candidate) { return (Resolve-Path -LiteralPath $candidate).Path }
    }
    return $null
}

# Each entry: executable, a function that builds arguments for one writer
# command line, and the process name whose memory is sampled.
$definitions = @{
    'buttonscli' = @{
        Exe = Find-Exe @($ButtonsCliPath)
        Args = { param($cmd) @('--tabs', '1', '--shell', 'cmd.exe /d', '--command', '1', $cmd) }
        Process = 'buttonscli'
        Note = 'command is typed after the shell settles; startup includes that delay'
    }
    'alacritty' = @{
        Exe = Find-Exe @('alacritty', "$env:ProgramFiles\Alacritty\alacritty.exe")
        Args = { param($cmd) @('-o', 'window.dimensions.columns=190', '-o', 'window.dimensions.lines=50', '-e', 'cmd.exe', '/d', '/c', $cmd) }
        Process = 'alacritty'
    }
    'wezterm' = @{
        Exe = Find-Exe @('wezterm', "$env:ProgramFiles\WezTerm\wezterm.exe")
        Args = { param($cmd) @('start', '--always-new-process', '--', 'cmd.exe', '/d', '/c', $cmd) }
        Process = 'wezterm-gui'
    }
    'ghostty' = @{
        Exe = Find-Exe @('ghostty')
        Args = { param($cmd) @('-e', 'cmd.exe', '/d', '/c', $cmd) }
        Process = 'ghostty'
    }
    'windows-terminal' = @{
        Exe = Find-Exe @('wt')
        Args = { param($cmd) @('-w', 'new', '--size', '190,50', 'cmd.exe', '/d', '/c', $cmd) }
        Process = 'WindowsTerminal'
        Note = 'memory is the shared WindowsTerminal process'
    }
    'conhost' = @{
        Exe = Find-Exe @("$env:SystemRoot\System32\conhost.exe")
        Args = { param($cmd) @('cmd.exe', '/d', '/c', $cmd) }
        Process = 'conhost'
        Note = 'legacy console baseline'
    }
}

function Stop-Tree([System.Diagnostics.Process]$Process) {
    if ($Process -and -not $Process.HasExited) {
        & taskkill.exe /PID $Process.Id /T /F 2>$null | Out-Null
    }
}

$results = New-Object System.Collections.Generic.List[object]
foreach ($name in $Terminals) {
    $definition = $definitions[$name]
    if (-not $definition) { Write-Warning "Unknown terminal '$name'"; continue }
    if (-not $definition.Exe) { Write-Warning "$name not found; skipped"; continue }
    Write-Host "== $name ($($definition.Exe))" -ForegroundColor Cyan

    foreach ($workload in $Workloads) {
        $file = Join-Path $workloadDir "$workload.vt"
        for ($run = 1; $run -le $Runs; $run++) {
            $out = Join-Path $workDir "$name-$workload-$run.json"
            $cmd = "`"$writer`" --cat `"$file`" --out `"$out`""
            $info = [System.Diagnostics.ProcessStartInfo]::new()
            $info.FileName = $definition.Exe
            foreach ($argument in (& $definition.Args $cmd)) { [void]$info.ArgumentList.Add($argument) }
            $info.UseShellExecute = $false
            if ($name -eq 'buttonscli') {
                $info.Environment['USERPROFILE'] = $isolatedHome
                $info.Environment['HOME'] = $isolatedHome
                $info.Environment['APPDATA'] = Join-Path $isolatedHome 'AppData\Roaming'
                $info.Environment['LOCALAPPDATA'] = Join-Path $isolatedHome 'AppData\Local'
                $info.Environment['BUTTONSCLI_NATIVE_DISABLE_REMOTE_CONFIG'] = '1'
                $info.Environment['BUTTONSCLI_NATIVE_DISABLE_ACCOUNT'] = '1'
            }
            $launchedMs = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
            $launch = [System.Diagnostics.Stopwatch]::StartNew()
            $process = [System.Diagnostics.Process]::Start($info)
            $windowMs = $null
            $peakMiB = 0.0
            try {
                while (-not (Test-Path -LiteralPath $out) -and $launch.Elapsed.TotalSeconds -lt $TimeoutSeconds) {
                    Start-Sleep -Milliseconds 50
                    if (-not $windowMs) {
                        $process.Refresh()
                        if (-not $process.HasExited -and $process.MainWindowHandle -ne 0) { $windowMs = $launch.ElapsedMilliseconds }
                    }
                    foreach ($p in Get-Process -Name $definition.Process -ErrorAction SilentlyContinue) {
                        $peakMiB = [Math]::Max($peakMiB, $p.WorkingSet64 / 1MB)
                    }
                }
                if (-not (Test-Path -LiteralPath $out)) {
                    Write-Warning "$name/$workload run $run timed out"
                    continue
                }
                Start-Sleep -Milliseconds 300
                $result = Get-Content -LiteralPath $out -Raw | ConvertFrom-Json
                $row = [pscustomobject]@{
                    terminal = $name
                    workload = $workload
                    run = $run
                    mib_per_s = [Math]::Round($result.bytes / 1MB / $result.seconds, 1)
                    seconds = [Math]::Round($result.seconds, 3)
                    first_window_ms = $windowMs
                    writer_started_ms = $result.started_unix_ms - $launchedMs
                    peak_working_set_mib = [Math]::Round($peakMiB, 1)
                }
                $results.Add($row)
                Write-Host ("  {0,-14} run {1}: {2,7:N1} MiB/s  window {3} ms  peak {4:N0} MiB" -f $workload, $run, $row.mib_per_s, $windowMs, $peakMiB)
            } finally {
                Stop-Tree $process
                Start-Sleep -Milliseconds 300
            }
        }
    }
    if ($definition.Note) { Write-Host "  note: $($definition.Note)" -ForegroundColor DarkGray }
}

Write-Host "`n== Median MiB/s (higher is better)" -ForegroundColor Cyan
$summary = $results | Group-Object terminal, workload | ForEach-Object {
    $sorted = $_.Group | Sort-Object mib_per_s
    $median = $sorted[[int][Math]::Floor($sorted.Count / 2)]
    [pscustomobject]@{
        terminal = $median.terminal
        workload = $median.workload
        median_mib_per_s = $median.mib_per_s
        first_window_ms = $median.first_window_ms
        peak_working_set_mib = ($_.Group | Measure-Object peak_working_set_mib -Maximum).Maximum
    }
}
$summary | Sort-Object workload, median_mib_per_s -Descending | Format-Table -AutoSize

if (-not $OutputPath) { $OutputPath = Join-Path $repoRoot "target\bench-terminals-$(Get-Date -Format 'yyyyMMdd-HHmmss').json" }
@{ machine = $env:COMPUTERNAME; mib = $MiB; runs = $Runs; results = $results; summary = $summary } |
    ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $OutputPath -Encoding utf8NoBOM
Write-Host "Saved $OutputPath"
Remove-Item -LiteralPath $workDir -Recurse -Force -ErrorAction SilentlyContinue
