use axiom_core::errors::{AxiomError, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{debug, error};

const SCROLLBACK_LINES: usize = 10_000;

/// A persistent PTY session wrapping `portable-pty`.
///
/// The shell process continues running even when the GUI is closed — it lives
/// inside the daemon process. GUI reconnect just tails the scrollback buffer.
pub struct PtySession {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    scrollback: Arc<Mutex<VecDeque<String>>>,
    pub cols: u16,
    pub rows: u16,
}

impl PtySession {
    /// Spawn a new persistent shell in the given working directory.
    pub async fn new(shell: &str, working_dir: &Path) -> Result<Self> {
        let pty_system = native_pty_system();
        let cols = 220u16;
        let rows = 50u16;

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| AxiomError::Pty(format!("openpty failed: {}", e)))?;

        let mut cmd = CommandBuilder::new(shell);
        cmd.cwd(working_dir);
        cmd.env("TERM", "xterm-256color");

        let _child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| AxiomError::Pty(format!("spawn failed: {}", e)))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| AxiomError::Pty(format!("take_writer failed: {}", e)))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| AxiomError::Pty(format!("try_clone_reader failed: {}", e)))?;

        let scrollback: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        let scrollback_bg = scrollback.clone();

        // Background reader task — collects output into the scrollback ring buffer
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut line_buf = String::new();
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                        for ch in chunk.chars() {
                            if ch == '\n' {
                                let mut sb = scrollback_bg.lock().unwrap();
                                sb.push_back(std::mem::take(&mut line_buf));
                                if sb.len() > SCROLLBACK_LINES {
                                    sb.pop_front();
                                }
                            } else {
                                line_buf.push(ch);
                            }
                        }
                    }
                    Err(e) => {
                        error!("PTY read error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            writer: Arc::new(Mutex::new(writer)),
            scrollback,
            cols,
            rows,
        })
    }

    /// Write user input to the terminal.
    pub async fn write_input(&mut self, input: &str) -> Result<()> {
        let mut w = self.writer.lock().map_err(|e| AxiomError::Pty(e.to_string()))?;
        w.write_all(input.as_bytes())
            .map_err(|e| AxiomError::Pty(format!("write failed: {}", e)))?;
        Ok(())
    }

    /// Resize the terminal.
    pub async fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.cols = cols;
        self.rows = rows;
        // portable-pty resize requires access to the master — stored in _pty_pair.
        // For now this is a no-op; proper resize requires storing master handle.
        Ok(())
    }

    /// Return the last N lines of scrollback.
    pub fn recent_output(&self, n: usize) -> Vec<String> {
        let sb = self.scrollback.lock().unwrap();
        let start = sb.len().saturating_sub(n);
        sb.iter().skip(start).cloned().collect()
    }
}
