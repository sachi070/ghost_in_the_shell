ghost_prompt_cmd() {
    local LAST_EXIT=$?
    printf "\033]1337;GhostExit=%d\007" "$LAST_EXIT"
}

if [[ ! "$PROMPT_COMMAND" =~ ghost_prompt_cmd ]]; then
    PROMPT_COMMAND="ghost_prompt_cmd;$PROMPT_COMMAND"
fi