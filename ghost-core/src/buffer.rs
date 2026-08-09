use std::collections::VecDeque;

#[derive(Debug, PartialEq)]
enum ParserState {
    Normal,
    Escape,
    Csi,
    Osc,
}

pub struct RollingBuffer {
    lines: VecDeque<String>,
    max_lines: usize,
    current_line: String,
    state: ParserState,
}

impl RollingBuffer {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(max_lines),
            max_lines,
            current_line: String::new(),
            state: ParserState::Normal,
        }
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            match self.state {
                ParserState::Normal => {
                    if byte == 0x1B {
                        self.state = ParserState::Escape;
                    } else if byte == b'\n' || byte == b'\r' {
                        let trimmed = self.current_line.trim().to_string();
                        if !trimmed.is_empty() {
                            if self.lines.len() >= self.max_lines {
                                self.lines.pop_front();
                            }
                            self.lines.push_back(trimmed);
                        }
                        self.current_line.clear();
                    } else if (32..=126).contains(&byte) {
                        self.current_line.push(byte as char);
                    }
                }
                ParserState::Escape => match byte {
                    b'[' => self.state = ParserState::Csi,
                    b']' => self.state = ParserState::Osc,
                    _ => self.state = ParserState::Normal,
                },
                ParserState::Csi => {
                    if (0x40..=0x7E).contains(&byte) {
                        self.state = ParserState::Normal;
                    }
                }
                ParserState::Osc => {
                    if byte == 0x07 || byte == 0x1B {
                        self.state = ParserState::Normal;
                    }
                }
            }
        }
    }

    pub fn get_context(&self) -> String {
        self.lines.iter().cloned().collect::<Vec<_>>().join("\n")
    }

    /// Extracts the clean executed command line strictly following prompt symbols ($ or >)
    pub fn extract_last_command(&self) -> String {
        for line in self.lines.iter().rev() {
            let trimmed = line.trim();

            // Skip empty lines, banners, headers, and shell error outputs
            if trimmed.is_empty()
                || trimmed.starts_with('[')
                || trimmed.starts_with("bash:")
                || trimmed.contains("Ghost Intercepted")
                || trimmed.contains("Ghost Doctor")
            {
                continue;
            }

            // Look specifically for prompt characters to isolate the typed command
            if let Some(pos) = trimmed.rfind('$') {
                let cmd = trimmed[pos + 1..].trim();
                if !cmd.is_empty() {
                    return cmd.to_string();
                }
            } else if let Some(pos) = trimmed.rfind('>') {
                let cmd = trimmed[pos + 1..].trim();
                if !cmd.is_empty() {
                    return cmd.to_string();
                }
            }
        }
        "unknown_command".to_string()
    }
}