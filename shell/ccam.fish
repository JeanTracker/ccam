# ccam shell integration for fish
# Add to ~/.config/fish/config.fish: ccam init fish | source

function ccam
  if test "$argv[1]" = "use"
    set alias $argv[2]
    if test -z "$alias"
      echo "Usage: ccam use <alias>" >&2
      return 1
    end
    set output (command ccam __env $alias 2>/tmp/ccam_err)
    set exit_code $status
    if test $exit_code -ne 0
      cat /tmp/ccam_err >&2
      return $exit_code
    end
    eval $output
    cat /tmp/ccam_err >&2
  else
    command ccam $argv
  end
end

# Tab completion
function __ccam_accounts
  command ccam list --names-only 2>/dev/null
end

complete -c ccam -f
complete -c ccam -n "__fish_use_subcommand use remove login logout status env" -a "(__ccam_accounts)"
complete -c ccam -n "not __fish_seen_subcommand_from add list remove use env login logout active status default keychain" \
  -a "add list remove use env login logout active status default keychain"

# Apply default account on new session
set _ccam_default (command ccam default --get 2>/dev/null)
if test -n "$_ccam_default"
  eval (command ccam __env $_ccam_default 2>/dev/null)
end
set -e _ccam_default

# Optional: show current ccam account in prompt
# Add to your fish_prompt function: echo -n (command ccam active --short 2>/dev/null | sed 's/.*/[ccam:&] /')
