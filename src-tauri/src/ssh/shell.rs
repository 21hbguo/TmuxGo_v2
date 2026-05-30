use ssh2::Channel;
use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

use super::connection::{HostConfig, SshConnection};

struct ShellInner {
    channel: Channel,
    _conn: SshConnection,
}

pub struct ShellManager {
    shells: Arc<Mutex<HashMap<String, ShellInner>>>,
}

impl ShellManager {
    pub fn new() -> Self {
        Self {
            shells: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start(
        &self,
        app: AppHandle,
        config: &HostConfig,
        session_id: String,
        cols: u32,
        rows: u32,
    ) -> Result<(), String> {
        let conn = SshConnection::connect(config)?;
        let session = conn.session();
        session.set_blocking(false);

        let mut channel = session.channel_session().map_err(|e| format!("Channel failed: {e}"))?;
        channel
            .request_pty("xterm-256color", None, Some((cols, rows, 0, 0)))
            .map_err(|e| format!("PTY request failed: {e}"))?;
        channel.shell().map_err(|e| format!("Shell failed: {e}"))?;

        let sid = session_id.clone();
        let app_out = app.clone();
        let reader_channel = channel.clone();

        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            let mut ch = reader_channel;
            loop {
                match ch.read(&mut buf) {
                    Ok(0) => {
                        if ch.eof() {
                            break;
                        }
                        thread::sleep(std::time::Duration::from_millis(20));
                    }
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).to_string();
                        let _ = app_out.emit(
                            "terminal-output",
                            serde_json::json!({
                                "data": data,
                                "sessionId": &sid,
                            }),
                        );
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(std::time::Duration::from_millis(20));
                        continue;
                    }
                    Err(_) => break,
                }
            }
            let _ = app_out.emit("terminal-closed", serde_json::json!({ "sessionId": sid }));
        });

        let mut shells = self.shells.lock().map_err(|e| format!("Lock poisoned: {e}"))?;
        shells.insert(session_id, ShellInner { channel, _conn: conn });
        Ok(())
    }

    pub fn write(&self, session_id: &str, data: &str) -> Result<(), String> {
        let mut shells = self.shells.lock().map_err(|e| format!("Lock poisoned: {e}"))?;
        let inner = shells.get_mut(session_id).ok_or("Shell not found")?;
        use std::io::Write;
        inner
            .channel
            .write_all(data.as_bytes())
            .map_err(|e| format!("Write failed: {e}"))?;
        Ok(())
    }

    pub fn resize(&self, session_id: &str, cols: u32, rows: u32) -> Result<(), String> {
        let mut shells = self.shells.lock().map_err(|e| format!("Lock poisoned: {e}"))?;
        let inner = shells.get_mut(session_id).ok_or("Shell not found")?;
        inner
            .channel
            .request_pty_size(cols, rows, None, None)
            .map_err(|e| format!("Resize failed: {e}"))?;
        Ok(())
    }

    pub fn close(&self, session_id: &str) -> Result<(), String> {
        let mut shells = self.shells.lock().map_err(|e| format!("Lock poisoned: {e}"))?;
        if let Some(mut inner) = shells.remove(session_id) {
            let _ = inner.channel.close();
        }
        Ok(())
    }

    pub fn list(&self) -> Vec<String> {
        self.shells
            .lock()
            .map(|s| s.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for ShellManager {
    fn default() -> Self {
        Self::new()
    }
}
