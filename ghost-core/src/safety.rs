#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RiskLevel {
    Safe,
    HighRisk,
    Critical,
}

pub fn evaluate_risk(cmd: &str) -> RiskLevel {
    let lower = cmd.trim().to_lowercase();

    // 1. CRITICAL: Catastrophic / Unrecoverable System-Wide Destruction (Hard blocked)
    if is_critical_system_destruction(&lower) {
        return RiskLevel::Critical;
    }

    // 2. HIGH RISK: Destructive / State Altering (Allowed, but requires typing CONFIRM)
    if lower.contains("rm -rf")
        || lower.contains("rm -r")
        || lower.contains("rm -f")
        || lower.contains("rmdir")
        || lower.contains("git reset --hard")
        || lower.contains("git clean -f")
        || lower.contains("git push -f")
        || lower.contains("git push --force")
        || lower.contains("git branch -d")
        || lower.contains("git branch -D")
        || lower.contains("drop table")
        || lower.contains("drop database")
        || lower.contains("truncate table")
        || lower.contains("kill -9")
        || lower.contains("pkill -9")
        || lower.contains("iptables -f")
        || lower.contains("chmod -r 777")
    {
        return RiskLevel::HighRisk;
    }

    // 3. SAFE: Standard non-destructive commands
    RiskLevel::Safe
}

fn is_critical_system_destruction(cmd: &str) -> bool {
    // Root or home directory obliteration
    if cmd.contains("rm -rf /")
        || cmd.contains("rm -rf /*")
        || cmd.contains("rm -rf ~")
        || cmd.contains("rm -rf $home")
        || cmd.contains("rm -rf /root")
        || cmd.contains("rm -rf c:\\")
        || cmd.contains("rm -rf c:/*")
        || cmd.contains("del /f /s /q c:\\")
    {
        return true;
    }

    // Disk formatting & block device overwrite
    if cmd.contains("mkfs.")
        || cmd.contains("mkfs ")
        || cmd.contains("dd if=/dev/zero")
        || cmd.contains("dd if=/dev/urandom")
        || cmd.contains("dd if=/dev/null")
        || (cmd.contains("dd ") && cmd.contains("of=/dev/sd"))
        || (cmd.contains("dd ") && cmd.contains("of=/dev/nvme"))
        || (cmd.contains("> /dev/sd") || cmd.contains("> /dev/nvme"))
    {
        return true;
    }

    // Fork bombs
    if cmd.contains(":(){ :|:& };:") || cmd.contains("forkbomb") {
        return true;
    }

    false
}