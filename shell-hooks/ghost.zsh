ghost_precmd() {
    local LAST_EXIT=$?
    printf "\033]1337;GhostExit=%d\007" "$LAST_EXIT"
}

autoload -Uz add-zsh-hook
add-zsh-hook precmd ghost_precmd