use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::ssh::connection::{HostConfig, SshConnection};

pub struct ConnectionPool {
    connections: Arc<Mutex<HashMap<String, SshConnection>>>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_or_connect(&self, config: &HostConfig) -> Result<(), String> {
        let mut conns = self.connections.lock().map_err(|e| format!("Lock poisoned: {e}"))?;
        if let Some(conn) = conns.get(&config.id) {
            if conn.is_alive() {
                return Ok(());
            }
            conns.remove(&config.id);
        }
        let conn = SshConnection::connect(config)?;
        conns.insert(config.id.clone(), conn);
        Ok(())
    }

    pub fn with_session<F, R>(&self, host_id: &str, f: F) -> Result<R, String>
    where
        F: FnOnce(&ssh2::Session) -> Result<R, String>,
    {
        let conns = self.connections.lock().map_err(|e| format!("Lock poisoned: {e}"))?;
        let conn = conns.get(host_id).ok_or_else(|| format!("No connection for host {host_id}"))?;
        if !conn.is_alive() {
            return Err(format!("Connection for host {host_id} is stale"));
        }
        f(conn.session())
    }

    pub fn disconnect(&self, host_id: &str) -> Result<(), String> {
        let mut conns = self.connections.lock().map_err(|e| format!("Lock poisoned: {e}"))?;
        conns.remove(host_id);
        Ok(())
    }

    pub fn disconnect_all(&self) {
        if let Ok(mut conns) = self.connections.lock() {
            conns.clear();
        }
    }

    pub fn list_connected(&self) -> Vec<String> {
        self.connections
            .lock()
            .map(|conns| conns.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn is_connected(&self, host_id: &str) -> bool {
        self.connections
            .lock()
            .map(|conns| conns.get(host_id).map_or(false, |c| c.is_alive()))
            .unwrap_or(false)
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}
