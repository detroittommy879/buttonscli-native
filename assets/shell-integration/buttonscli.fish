# ButtonsCLI shell integration for fish: OSC 133 marks (A prompt start,
# C command start, D;<exit code> finished). ButtonsCLI loads it with
# --init-command after your config. Over SSH: source it from config.fish.
if not set -q __bcli_loaded; and status is-interactive
    set -g __bcli_loaded 1
    function __bcli_prompt --on-event fish_prompt
        printf '\e]133;A\a'
    end
    function __bcli_preexec --on-event fish_preexec
        printf '\e]133;C\a'
    end
    function __bcli_postexec --on-event fish_postexec
        printf '\e]133;D;%s\a' $status
    end
end
