# ccam shell integration for bash
# Add to ~/.bashrc: eval "$(ccam init bash)"

ccam() {
  local cmd="${1:-}"
  if [[ "$cmd" == "use" ]]; then
    local alias="${2:-}"
    if [[ -z "$alias" ]]; then
      echo "Usage: ccam use <alias>" >&2
      return 1
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
  else
    command ccam "$@"
  fi
}

# Tab completion
if [[ -n "$BASH_VERSION" ]]; then
  _ccam_complete() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local prev="${COMP_WORDS[COMP_CWORD-1]}"
    case "$prev" in
      use|remove|login|logout|status|env)
        local accounts
        accounts="$(command ccam list --names-only 2>/dev/null)"
        COMPREPLY=($(compgen -W "$accounts" -- "$cur"))
        ;;
      *)
        COMPREPLY=($(compgen -W "add list remove use env login logout active status default keychain" -- "$cur"))
        ;;
    esac
  }
  complete -F _ccam_complete ccam
fi

# Apply default account on new session
_ccam_default="$(command ccam default --get 2>/dev/null)"
if [[ -n "$_ccam_default" ]]; then
  eval "$(command ccam __env "$_ccam_default" 2>/dev/null)"
fi
unset _ccam_default

# Optional: show current ccam account in prompt
# Uncomment to enable:
# PS1="\$(command ccam active --short 2>/dev/null | sed 's/.*/[ccam:&] /') $PS1"
