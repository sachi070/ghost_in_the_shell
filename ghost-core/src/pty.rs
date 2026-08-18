use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Bash,
    Zsh,
    PowerShell,
    Cmd,
    GenericPosix,
}

pub struct ShellPty {
    pub pair: PtyPair,
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
    pub shell_kind: ShellKind,
}

impl ShellPty {
    pub fn spawn() -> Result<Self> {
        let pty_system = native_pty_system();

        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to initialize system PTY")?;

        let (shell_cmd, shell_kind) = Self::detect_active_shell();
        let mut cmd = CommandBuilder::new(&shell_cmd);

        for (key, value) in env::vars() {
            cmd.env(key, value);
        }

        cmd.env("GHOST_SHELL", "1");

        // Set matching OSC 1337 exit hook per shell environment
        match shell_kind {
            ShellKind::Bash | ShellKind::GenericPosix => {
                cmd.env(
                    "PROMPT_COMMAND",
                    r#"printf "\033]1337;GhostExit=%d\007" "$?""#,
                );
            }
            ShellKind::Zsh => {
                cmd.env(
                    "PROMPT_COMMAND",
                    r#"printf "\033]1337;GhostExit=%d\007" "$?""#,
                );
            }
            _ => {}
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .with_context(|| format!("Failed to spawn shell process: {}", shell_cmd))?;

        Ok(Self {
            pair,
            child,
            shell_kind,
        })
    }

    fn detect_active_shell() -> (String, ShellKind) {
        if let Ok(target) = env::var("GHOST_TARGET_SHELL") {
            let lower = target.to_lowercase();
            if lower.contains("zsh") {
                return (target, ShellKind::Zsh);
            } else if lower.contains("pwsh") || lower.contains("powershell") {
                return (target, ShellKind::PowerShell);
            } else if lower.contains("bash") {
                return (target, ShellKind::Bash);
            } else if lower.contains("cmd") {
                return (target, ShellKind::Cmd);
            }
        }

        if cfg!(windows) {
            if env::var("MSYSTEM").is_ok() || env::var("MINGW_PREFIX").is_ok() {
                if let Ok(shell) = env::var("SHELL") {
                    return (shell, ShellKind::Bash);
                }
                let git_bash = r"C:\Program Files\Git\bin\bash.exe";
                if std::path::Path::new(git_bash).exists() {
                    return (git_bash.to_string(), ShellKind::Bash);
                }
            }

            if env::var("PSModulePath").is_ok() {
                if Self::is_in_path("pwsh.exe") {
                    return ("pwsh.exe".to_string(), ShellKind::PowerShell);
                }
                if Self::is_in_path("powershell.exe") {
                    return ("powershell.exe".to_string(), ShellKind::PowerShell);
                }
            }

            let comspec = env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
            return (comspec, ShellKind::Cmd);
        }

        if let Ok(shell_var) = env::var("SHELL") {
            if shell_var.ends_with("/zsh") {
                return (shell_var, ShellKind::Zsh);
            } else if shell_var.ends_with("/bash") {
                return (shell_var, ShellKind::Bash);
            } else {
                return (shell_var, ShellKind::GenericPosix);
            }
        }

        if Self::is_in_path("zsh") {
            ("zsh".to_string(), ShellKind::Zsh)
        } else if Self::is_in_path("bash") {
            ("bash".to_string(), ShellKind::Bash)
        } else {
            ("/bin/sh".to_string(), ShellKind::GenericPosix)
        }
    }

    fn is_in_path(binary: &str) -> bool {
        if let Ok(path_var) = env::var("PATH") {
            let sep = if cfg!(windows) { ';' } else { ':' };
            for p in path_var.split(sep) {
                if std::path::Path::new(p).join(binary).is_file() {
                    return true;
                }
            }
        }
        false
    }

    pub fn get_alias_init_payload(&self) -> Option<&'static [u8]> {
        match self.shell_kind {
            ShellKind::Bash | ShellKind::Zsh | ShellKind::GenericPosix => Some(
                b"alias ghost=':' f=':' fix=':' y=':' yes=':' Y=':' YES=':' n=':' no=':' N=':' NO=':' confirm=':' CONFIRM=':' 2>/dev/null; PROMPT_COMMAND='printf \"\\033]1337;GhostExit=%d\\007\" \"$?\"'; clear\r",
            ),
            ShellKind::PowerShell => Some(
                b"$oldPrompt = $function:prompt; function prompt { $code = if ($LASTEXITCODE -ne $null) { $LASTEXITCODE } else { 0 }; [Console]::Write(\"`e]1337;GhostExit=$code`a\"); if ($oldPrompt) { & $oldPrompt } else { 'PS ' + (Get-Location) + '> ' } }; function global:ghost {}; function global:f {}; function global:fix {}; function global:y {}; function global:yes {}; function global:n {}; function global:no {}; function global:confirm {}; Clear-Host\r",
            ),
            ShellKind::Cmd => None,
        }
    }
}