use anyhow::Result;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

pub struct RawModeGuard;

impl RawModeGuard {
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}