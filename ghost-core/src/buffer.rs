use std::collections::VecDeque;

pub struct RollingBuffer {
    max_lines: usize,
    lines: VecDeque<String>,
}

impl RollingBuffer {
    pub fn new(max_lines: usize) -> Self {
        Self {
            max_lines,
            lines: VecDeque::with_capacity(max_lines),
        }
    }

    /// Push new raw output bytes into the rolling line buffer
    pub fn push_bytes(&mut self, bytes: &[u8]) {
        let text = String::from_utf8_lossy(bytes);
        for line in text.lines() {
            if self.lines.len() >= self.max_lines {
                self.lines.pop_front();
            }
            self.lines.push_back(line.to_string());
        }
    }

    /// Retrieve captured console context as a single text block
    pub fn get_context(&self) -> String {
        self.lines.iter().cloned().collect::<Vec<_>>().join("\n")
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.lines.clear();
    }
}