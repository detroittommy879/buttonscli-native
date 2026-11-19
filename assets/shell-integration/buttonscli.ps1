# ButtonsCLI shell integration for PowerShell (5.1 and 7+).
# Sends OSC 133 marks: A prompt start, B input start, C command start,
# D;<exit code> command finished, and 633;E with the command line.
# ButtonsCLI runs this after your profile, so it wraps your own prompt.
# To use it over SSH or in a shell ButtonsCLI did not start: . ./buttonscli.ps1
if (-not $Global:__bcliLoaded) {
    $Global:__bcliLoaded = $true
    $Global:__bcliRan = $false
    $Global:__bcliCodeBefore = $null
    $Global:__bcliOriginalPrompt = $function:prompt

    function Global:__bcliWrapReadLine {
        if ($Global:__bcliReadLine -or -not (Test-Path Function:\PSConsoleHostReadLine)) {
            return
        }
        $Global:__bcliReadLine = $function:PSConsoleHostReadLine
        function Global:PSConsoleHostReadLine {
            $line = & $Global:__bcliReadLine
            if ($line -and $line.Trim()) {
                $Global:__bcliRan = $true
                $Global:__bcliCodeBefore = $Global:LASTEXITCODE
                $e = [char]27; $b = [char]7
                $escaped = $line -replace '\\', '\\' -replace ';', '\x3b' -replace "`r", '\x0d' -replace "`n", '\x0a'
                [Console]::Write("$e]633;E;$escaped$b$e]133;C$b")
            }
            $line
        }
    }

    function Global:prompt {
        $ok = $?
        $code = $Global:LASTEXITCODE
        $e = [char]27; $b = [char]7
        $marks = ''
        if ($Global:__bcliRan) {
            $status = 0
            if (-not $ok) {
                $status = 1
                if ($code -is [int] -and $code -ne 0 -and $code -ne $Global:__bcliCodeBefore) {
                    $status = $code
                }
            }
            $marks += "$e]133;D;$status$b"
            $Global:__bcliRan = $false
        }
        $marks += "$e]133;A$b"
        # PSReadLine may load after this script: wrap its reader on first use.
        __bcliWrapReadLine
        $text = if ($Global:__bcliOriginalPrompt) { & $Global:__bcliOriginalPrompt } else { "PS $($executionContext.SessionState.Path.CurrentLocation)> " }
        $Global:LASTEXITCODE = $code
        "$marks$text$e]133;B$b"
    }
}
