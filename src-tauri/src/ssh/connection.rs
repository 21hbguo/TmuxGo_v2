use ssh2::Session;
use std::net::TcpStream;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HostConfig {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: AuthMethod,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AuthMethod {
    Password { password: String },
    KeyFile { path: String, passphrase: Option<String> },
    Agent,
}

pub struct SshConnection {
    session: Session,
    _tcp: TcpStream,
    pub host_id: String,
    pub connected_at: Instant,
}

impl SshConnection {
    pub fn connect(config: &HostConfig) -> Result<Self, String> {
        let addr = format!("{}:{}", config.host, config.port);
        let tcp = TcpStream::connect(&addr).map_err(|e| format!("TCP connect failed: {e}"))?;
        tcp.set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| format!("Set read timeout failed: {e}"))?;

        let mut session = Session::new().map_err(|e| format!("Session create failed: {e}"))?;
        session.set_tcp_stream(tcp.try_clone().map_err(|e| format!("TCP clone failed: {e}"))?);
        session
            .handshake()
            .map_err(|e| format!("SSH handshake failed: {e}"))?;

        match &config.auth {
            AuthMethod::Password { password } => {
                session
                    .userauth_password(&config.username, password)
                    .map_err(|e| format!("Password auth failed: {e}"))?;
            }
            AuthMethod::KeyFile { path, passphrase } => {
                session
                    .userauth_pubkey_file(
                        &config.username,
                        None,
                        std::path::Path::new(path),
                        passphrase.as_deref(),
                    )
                    .map_err(|e| format!("Key auth failed: {e}"))?;
            }
            AuthMethod::Agent => {
                let mut agent = session.agent().map_err(|e| format!("Agent connect failed: {e}"))?;
                agent.connect().map_err(|e| format!("Agent connect failed: {e}"))?;
                agent
                    .list_identities()
                    .map_err(|e| format!("Agent list identities failed: {e}"))?;
                let identities = agent.identities().map_err(|e| format!("Agent identities failed: {e}"))?;
                let mut authed = false;
                for identity in identities {
                    if agent.userauth(&config.username, &identity).is_ok() {
                        authed = true;
                        break;
                    }
                }
                if !authed {
                    return Err("SSH agent auth failed: no valid identity".into());
                }
            }
        }

        if !session.authenticated() {
            return Err("SSH authentication failed".into());
        }

        Ok(Self {
            session,
            _tcp: tcp,
            host_id: config.id.clone(),
            connected_at: Instant::now(),
        })
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn is_alive(&self) -> bool {
        self.session.authenticated() && self.connected_at.elapsed() < Duration::from_secs(3600)
    }

    pub fn keepalive(&self) -> Result<(), String> {
        self.session
            .keepalive_send()
            .map_err(|e| format!("Keepalive failed: {e}"))?;
        Ok(())
    }
}

impl Drop for SshConnection {
    fn drop(&mut self) {
        let _ = self.session.disconnect(None, "client closing", None);
    }
}
