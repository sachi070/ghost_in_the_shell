use std::collections::VecDeque;

pub struct RollingBuffer {
    lines: VecDeque<String>,
    max_lines: usize,
    current_line: String,
}

impl RollingBuffer {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(max_lines),
            max_lines,
            current_line: String::new(),
        }
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if byte == b'\n' || byte == b'\r' {
                if !self.current_line.trim().is_empty() {
                    if self.lines.len() >= self.max_lines {
                        self.lines.pop_front();
                    }
                    self.lines.push_back(self.current_line.clone());
                }
                self.current_line.clear();
            } else if byte >= 32 && byte <= 126 {
                self.current_line.push(byte as char);
            }
        }
    }

    pub fn get_context(&self) -> String {
        self.lines.iter().cloned().collect::<Vec<_>>().join("\n")
    }

    /// Extracts the most recently executed command line prior to error interception
    pub fn extract_last_command(&self) -> String {
        for line in self.lines.iter().rev() {
            let trimmed = line.trim();
            // Filter out empty lines, prompt indicators, and ghost headers
            if !trimmed.is_empty() 
                && !trimmed.starts_with('[') 
                && !trimmed.contains("Ghost Intercepted") 
            {
                // Strip prompt prefix if present (e.g. "hp@DESKTOP... $ cat foo" -> "cat foo")
                if let Some(pos) = trimmed.rfind('$') {
                    return trimmed[pos + 1..].trim().to_string();
                } else if let Some(pos) = trimmed.rfind('>') {
                    return trimmed[pos + 1..].trim().to_string();
                }
                return trimmed.to_string();
            }
        }
        "unknown_command".to_string()
    }
}