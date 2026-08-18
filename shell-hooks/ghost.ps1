$oldPrompt = $function:prompt
function prompt {
    $code = if ($LASTEXITCODE -ne $null) { $LASTEXITCODE } else { 0 }
    [Console]::Write("`e]1337;GhostExit=$code`a")
    if ($oldPrompt) { & $oldPrompt } else { "PS $($ExecutionContext.SessionState.Path.CurrentLocation)> " }
}

function global:f {}
function global:fix {}
function global:y {}
function global:yes {}
function global:n {}
function global:no {}
function global:confirm {}