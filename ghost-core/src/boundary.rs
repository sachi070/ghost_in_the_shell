pub struct BoundaryParser {
    buffer: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CommandStatus {
    Finished { exit_code: i32 },
    Running,
}

impl BoundaryParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Process incoming raw bytes from PTY output and check for GhostExit markers
    pub fn parse_bytes(&mut self, bytes: &[u8]) -> CommandStatus {
        let text = String::from_utf8_lossy(bytes);
        self.buffer.push_str(&text);

        // Look for pattern: \x1b]1337;GhostExit=X\x07
        if let Some(start_idx) = self.buffer.find("\x1b]1337;GhostExit=") {
            let offset = start_idx + "\x1b]1337;GhostExit=".len();
            if let Some(end_idx) = self.buffer[offset..].find('\x07') {
                let code_str = &self.buffer[offset..offset + end_idx];
                let exit_code = code_str.parse::<i32>().unwrap_or(0);

                // Drain the processed boundary signal from memory
                self.buffer.clear();
                return CommandStatus::Finished { exit_code };
            }
        }

        // Keep buffer size manageable (rolling max 4096 chars)
        if self.buffer.len() > 4096 {
            let drain_amount = self.buffer.len() - 2048;
            self.buffer.drain(..drain_amount);
        }

        CommandStatus::Running
    }
}