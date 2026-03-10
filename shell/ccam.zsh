# ccam shell integration for zsh
# Add to ~/.zshrc: eval "$(ccam init zsh)"

function ccam() {
  local cmd="${1:-}"
  if [[ "$cmd" == "use" ]]; then
    local alias="${2:-}"
    if [[ -z "$alias" ]]; then
      printf "error: the following required arguments were not provided:\n  <ALIAS>\n\nUsage: ccam use <ALIAS>\n\nFor more information, try '--help'.\n" >&2
      return 1
    fi
    if [[ "$alias" == -* ]]; then
      command ccam "$@"
      return $?
    fi
    local output
    output="$(command ccam __env "$alias" 2>/tmp/ccam_err)"
    local exit_code=$?
    if [[ $exit_code -ne 0 ]]; then
      cat /tmp/ccam_err >&2
      return $exit_code
    fi
    eval "$output"
    cat /tmp/ccam_err >&2
  elif [[ "$cmd" == "remove" || "$cmd" == "rm" ]]; then
    local output
    output="$(command ccam "$@" 2>/tmp/ccam_err)"
    local exit_code=$?
    cat /tmp/ccam_err >&2
    if [[ $exit_code -eq 0 && -n "$output" ]]; then
      eval "$output"
    fi
    return $exit_code
  else
    command ccam "$@"
  fi
}

# Tab completion
if (( $+functions[compdef] )); then
  _ccam_complete() {
    local -a accounts
    accounts=(${(f)"$(command ccam list --names-only 2>/dev/null)"})
    local -a cmds
    cmds=(add list remove use active status default keychain)
    case "$words[2]" in
      use|remove|status)
        _describe 'accounts' accounts
        ;;
      *)
        _describe 'commands' cmds
        ;;
    esac
  }
  compdef _ccam_complete ccam
fi

# Apply default account on new session (only if default is set)
_ccam_default="$(command ccam default --get 2>/dev/null)"
if [[ -n "$_ccam_default" ]]; then
  eval "$(command ccam __env --no-refresh "$_ccam_default" 2>/dev/null)"
fi
unset _ccam_default

# Optional: show current ccam account in right prompt
# Uncomment to enable:
# _ccam_prompt() { command ccam active --short 2>/dev/null; }
# RPROMPT='%F{cyan}[ccam:$(_ccam_prompt)]%f $RPROMPT'
