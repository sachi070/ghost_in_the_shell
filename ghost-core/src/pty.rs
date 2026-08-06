use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::env;

pub struct ShellPty {
    pub master: Box<dyn MasterPty + Send>,
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl ShellPty {
    pub fn spawn() -> Result<Self> {
        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to allocate PTY pair")?;

        let shell = env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(windows) {
                "cmd.exe".to_string()
            } else {
                "/bin/bash".to_string()
            }
        });

        let mut cmd = CommandBuilder::new(&shell);

        if let Ok(path_val) = env::var("PATH") {
            cmd.env("PATH", path_val);
        }

        cmd.env("TERM", "xterm-256color");
        cmd.env("GHOST_INSIDE", "1");
        cmd.env(
            "PROMPT_COMMAND",
            "printf \"\\033]1337;GhostExit=%d\\007\" \"$?\"",
        );

        let child = pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn shell into PTY slave")?;

        Ok(Self {
            master: pair.master,
            child,
        })
    }
}