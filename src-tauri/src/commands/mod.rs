use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

use crate::ssh::exec;
use crate::ssh::pool::ConnectionPool;
use crate::ssh::shell::ShellManager;

pub struct AppState {
    pub pool: ConnectionPool,
    pub shell_manager: ShellManager,
    pub hosts_config_path: std::path::PathBuf,
    pub preferences_path: std::path::PathBuf,
}

// ── Hosts ──

#[derive(Serialize, Deserialize, Clone)]
pub struct HostEntry {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String,
    pub password: Option<String>,
    pub key_path: Option<String>,
    pub key_passphrase: Option<String>,
    pub group: Option<String>,
    pub tags: Option<Vec<String>>,
}

fn load_hosts(path: &std::path::Path) -> Vec<HostEntry> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_hosts(path: &std::path::Path, hosts: &[HostEntry]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(hosts).map_err(|e| format!("Serialize failed: {e}"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Create dir failed: {e}"))?;
    }
    std::fs::write(path, json).map_err(|e| format!("Write failed: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn list_hosts(state: State<'_, Arc<AppState>>) -> Result<Vec<HostEntry>, String> {
    Ok(load_hosts(&state.hosts_config_path))
}

#[tauri::command]
pub fn get_host(state: State<'_, Arc<AppState>>, id: String) -> Result<HostEntry, String> {
    load_hosts(&state.hosts_config_path)
        .into_iter()
        .find(|h| h.id == id)
        .ok_or_else(|| format!("Host {id} not found"))
}

#[tauri::command]
pub fn create_host(state: State<'_, Arc<AppState>>, host: HostEntry) -> Result<HostEntry, String> {
    let mut hosts = load_hosts(&state.hosts_config_path);
    if hosts.iter().any(|h| h.id == host.id) {
        return Err(format!("Host {} already exists", host.id));
    }
    hosts.push(host.clone());
    save_hosts(&state.hosts_config_path, &hosts)?;
    Ok(host)
}

#[tauri::command]
pub fn update_host(state: State<'_, Arc<AppState>>, id: String, host: HostEntry) -> Result<HostEntry, String> {
    let mut hosts = load_hosts(&state.hosts_config_path);
    let idx = hosts.iter().position(|h| h.id == id).ok_or_else(|| format!("Host {id} not found"))?;
    hosts[idx] = host.clone();
    save_hosts(&state.hosts_config_path, &hosts)?;
    Ok(host)
}

#[tauri::command]
pub fn delete_host(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    let mut hosts = load_hosts(&state.hosts_config_path);
    hosts.retain(|h| h.id != id);
    save_hosts(&state.hosts_config_path, &hosts)?;
    state.pool.disconnect(&id).ok();
    Ok(())
}

#[tauri::command]
pub fn connect_host(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    let host = load_hosts(&state.hosts_config_path)
        .into_iter()
        .find(|h| h.id == id)
        .ok_or_else(|| format!("Host {id} not found"))?;

    let auth = match host.auth_type.as_str() {
        "key" => crate::ssh::connection::AuthMethod::KeyFile {
            path: host.key_path.unwrap_or_default(),
            passphrase: host.key_passphrase,
        },
        "agent" => crate::ssh::connection::AuthMethod::Agent,
        _ => crate::ssh::connection::AuthMethod::Password {
            password: host.password.unwrap_or_default(),
        },
    };

    let config = crate::ssh::connection::HostConfig {
        id: host.id,
        name: host.name,
        host: host.host,
        port: host.port,
        username: host.username,
        auth,
    };

    state.pool.get_or_connect(&config)
}

// ── Sessions ──

#[tauri::command]
pub fn list_sessions(state: State<'_, Arc<AppState>>, host_id: String) -> Result<serde_json::Value, String> {
    let output = state.pool.with_session(&host_id, |session| {
        exec::exec(session, "tmux list-sessions -F '#{session_id},#{session_name},#{session_attached},#{session_windows}'")
    })?;
    let (stdout, _, _) = output;
    let sessions: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.splitn(4, ',').collect();
            serde_json::json!({
                "id": parts.first().unwrap_or(&""),
                "name": parts.get(1).unwrap_or(&""),
                "attached": parts.get(2).unwrap_or(&"0") == &"1",
                "windows": parts.get(3).unwrap_or(&"0").parse::<u32>().unwrap_or(0),
            })
        })
        .collect();
    Ok(serde_json::to_value(sessions).unwrap())
}

#[tauri::command]
pub fn create_session(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    name: String,
    _layout: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let cmd = format!("tmux new-session -d -s '{}' -P -F '{{\"id\":\"#{{session_id}}\",\"name\":\"#{{session_name}}\"}}'", name.replace('\'', "'\\''"));
    let output = state.pool.with_session(&host_id, |session| exec::exec(session, &cmd))?;
    let (stdout, _, _) = output;
    serde_json::from_str(stdout.trim()).map_err(|_| "Session created".to_string())
}

#[tauri::command]
pub fn rename_session(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    session_id: String,
    name: String,
) -> Result<(), String> {
    let cmd = format!("tmux rename-session -t '{}' '{}'", session_id.replace('\'', "'\\''"), name.replace('\'', "'\\''"));
    state.pool.with_session(&host_id, |session| {
        exec::exec(session, &cmd)?;
        Ok(())
    })
}

#[tauri::command]
pub fn delete_session(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    session_id: String,
) -> Result<(), String> {
    let cmd = format!("tmux kill-session -t '{}'", session_id.replace('\'', "'\\''"));
    state.pool.with_session(&host_id, |session| {
        exec::exec(session, &cmd)?;
        Ok(())
    })
}

// ── Windows ──

#[tauri::command]
pub fn list_windows(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    session_id: String,
) -> Result<serde_json::Value, String> {
    let cmd = format!(
        "tmux list-windows -t '{}' -F '#{{window_id}},#{{window_name}},#{{window_active}},#{{window_panes}}'",
        session_id.replace('\'', "'\\''")
    );
    let output = state.pool.with_session(&host_id, |session| exec::exec(session, &cmd))?;
    let (stdout, _, _) = output;
    let windows: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.splitn(4, ',').collect();
            serde_json::json!({
                "id": parts.first().unwrap_or(&""),
                "name": parts.get(1).unwrap_or(&""),
                "active": parts.get(2).unwrap_or(&"0") == &"1",
                "panes": parts.get(3).unwrap_or(&"0").parse::<u32>().unwrap_or(0),
            })
        })
        .collect();
    Ok(serde_json::to_value(windows).unwrap())
}

#[tauri::command]
pub fn create_window(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    session_id: String,
    name: String,
) -> Result<serde_json::Value, String> {
    let cmd = format!(
        "tmux new-window -t '{}' -n '{}' -P -F '{{\"id\":\"#{{window_id}}\",\"name\":\"#{{window_name}}\"}}'",
        session_id.replace('\'', "'\\''"),
        name.replace('\'', "'\\''")
    );
    let output = state.pool.with_session(&host_id, |session| exec::exec(session, &cmd))?;
    let (stdout, _, _) = output;
    serde_json::from_str(stdout.trim()).map_err(|_| "Window created".to_string())
}

#[tauri::command]
pub fn select_window(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    session_id: String,
    window_id: String,
) -> Result<(), String> {
    let cmd = format!(
        "tmux select-window -t '{}' -'{}'",
        session_id.replace('\'', "'\\''"),
        window_id.replace('\'', "'\\''")
    );
    state.pool.with_session(&host_id, |session| {
        exec::exec(session, &cmd)?;
        Ok(())
    })
}

#[tauri::command]
pub fn rename_window(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    session_id: String,
    window_id: String,
    name: String,
) -> Result<(), String> {
    let cmd = format!(
        "tmux rename-window -t '{}':'{}' '{}'",
        session_id.replace('\'', "'\\''"),
        window_id.replace('\'', "'\\''"),
        name.replace('\'', "'\\''")
    );
    state.pool.with_session(&host_id, |session| {
        exec::exec(session, &cmd)?;
        Ok(())
    })
}

#[tauri::command]
pub fn move_windows(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    session_id: String,
    ordered_window_ids: Vec<String>,
) -> Result<(), String> {
    for (i, wid) in ordered_window_ids.iter().enumerate() {
        let cmd = format!(
            "tmux move-window -s '{}' -t '{}:{}'",
            wid.replace('\'', "'\\''"),
            session_id.replace('\'', "'\\''"),
            i
        );
        state.pool.with_session(&host_id, |session| {
            exec::exec(session, &cmd)?;
            Ok(())
        })?;
    }
    Ok(())
}

#[tauri::command]
pub fn kill_window(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    session_id: String,
    window_id: String,
) -> Result<(), String> {
    let cmd = format!(
        "tmux kill-window -t '{}':'{}'",
        session_id.replace('\'', "'\\''"),
        window_id.replace('\'', "'\\''")
    );
    state.pool.with_session(&host_id, |session| {
        exec::exec(session, &cmd)?;
        Ok(())
    })
}

// ── Panes ──

#[tauri::command]
pub fn list_panes(
    state: State<'_, Arc<AppState>>,
    window_id: String,
) -> Result<serde_json::Value, String> {
    let cmd = format!(
        "tmux list-panes -t '{}' -F '#{{pane_id}},#{{pane_title}},#{{pane_width}},#{{pane_height}},#{{pane_active}}'",
        window_id.replace('\'', "'\\''")
    );
    // We need a host_id but the frontend doesn't pass it for list_panes.
    // Use the first connected host.
    let connected = state.pool.list_connected();
    let host_id = connected.first().ok_or("No connected host")?;
    let output = state.pool.with_session(host_id, |session| exec::exec(session, &cmd))?;
    let (stdout, _, _) = output;
    let panes: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.splitn(5, ',').collect();
            serde_json::json!({
                "id": parts.first().unwrap_or(&""),
                "title": parts.get(1).unwrap_or(&""),
                "width": parts.get(2).unwrap_or(&"0").parse::<u32>().unwrap_or(0),
                "height": parts.get(3).unwrap_or(&"0").parse::<u32>().unwrap_or(0),
                "active": parts.get(4).unwrap_or(&"0") == &"1",
            })
        })
        .collect();
    Ok(serde_json::to_value(panes).unwrap())
}

#[tauri::command]
pub fn list_session_panes(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    session_id: String,
) -> Result<serde_json::Value, String> {
    let cmd = format!(
        "tmux list-panes -t '{}' -s -F '#{{pane_id}},#{{pane_title}},#{{pane_width}},#{{pane_height}},#{{pane_active}},#{{window_id}}'",
        session_id.replace('\'', "'\\''")
    );
    let output = state.pool.with_session(&host_id, |session| exec::exec(session, &cmd))?;
    let (stdout, _, _) = output;
    let panes: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.splitn(6, ',').collect();
            serde_json::json!({
                "id": parts.first().unwrap_or(&""),
                "title": parts.get(1).unwrap_or(&""),
                "width": parts.get(2).unwrap_or(&"0").parse::<u32>().unwrap_or(0),
                "height": parts.get(3).unwrap_or(&"0").parse::<u32>().unwrap_or(0),
                "active": parts.get(4).unwrap_or(&"0") == &"1",
                "windowId": parts.get(5).unwrap_or(&""),
            })
        })
        .collect();
    Ok(serde_json::to_value(panes).unwrap())
}

#[tauri::command]
pub fn get_pane_output(
    state: State<'_, Arc<AppState>>,
    pane_id: String,
) -> Result<serde_json::Value, String> {
    let cmd = format!("tmux capture-pane -t '{}' -p", pane_id.replace('\'', "'\\''"));
    let connected = state.pool.list_connected();
    let host_id = connected.first().ok_or("No connected host")?;
    let output = state.pool.with_session(host_id, |session| exec::exec(session, &cmd))?;
    let (stdout, _, _) = output;
    Ok(serde_json::json!({
        "paneId": pane_id,
        "data": stdout,
    }))
}

#[tauri::command]
pub fn split_pane(
    state: State<'_, Arc<AppState>>,
    pane_id: String,
    direction: String,
) -> Result<serde_json::Value, String> {
    let flag = if direction == "horizontal" { "-h" } else { "-v" };
    let cmd = format!(
        "tmux split-window {} -t '{}' -P -F '{{\"id\":\"#{{pane_id}}\"}}'",
        flag,
        pane_id.replace('\'', "'\\''")
    );
    let connected = state.pool.list_connected();
    let host_id = connected.first().ok_or("No connected host")?;
    let output = state.pool.with_session(host_id, |session| exec::exec(session, &cmd))?;
    let (stdout, _, _) = output;
    serde_json::from_str(stdout.trim()).map_err(|_| "Pane split".to_string())
}

#[tauri::command]
pub fn zoom_pane(
    state: State<'_, Arc<AppState>>,
    pane_id: Option<String>,
) -> Result<(), String> {
    let target = pane_id.as_deref().unwrap_or("");
    let cmd = if target.is_empty() {
        "tmux resize-pane -Z".to_string()
    } else {
        format!("tmux resize-pane -Z -t '{}'", target.replace('\'', "'\\''"))
    };
    let connected = state.pool.list_connected();
    let host_id = connected.first().ok_or("No connected host")?;
    state.pool.with_session(host_id, |session| {
        exec::exec(session, &cmd)?;
        Ok(())
    })
}

#[tauri::command]
pub fn select_pane(
    state: State<'_, Arc<AppState>>,
    pane_id: String,
) -> Result<(), String> {
    let cmd = format!("tmux select-pane -t '{}'", pane_id.replace('\'', "'\\''"));
    let connected = state.pool.list_connected();
    let host_id = connected.first().ok_or("No connected host")?;
    state.pool.with_session(host_id, |session| {
        exec::exec(session, &cmd)?;
        Ok(())
    })
}

#[tauri::command]
pub fn kill_pane(
    state: State<'_, Arc<AppState>>,
    pane_id: Option<String>,
) -> Result<(), String> {
    let target = pane_id.as_deref().unwrap_or("");
    let cmd = if target.is_empty() {
        "tmux kill-pane".to_string()
    } else {
        format!("tmux kill-pane -t '{}'", target.replace('\'', "'\\''"))
    };
    let connected = state.pool.list_connected();
    let host_id = connected.first().ok_or("No connected host")?;
    state.pool.with_session(host_id, |session| {
        exec::exec(session, &cmd)?;
        Ok(())
    })
}

// ── Snapshot ──

#[tauri::command]
pub fn get_snapshot(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    session_id: String,
) -> Result<serde_json::Value, String> {
    let session_cmd = format!(
        "tmux list-sessions -F '#{{session_id}},#{{session_name}}' -f '#{{==:#{},#{{session_id}}}}'",
        session_id.replace('\'', "'\\''")
    );
    let windows_cmd = format!(
        "tmux list-windows -t '{}' -F '#{{window_id}},#{{window_name}},#{{window_active}},#{{window_panes}}'",
        session_id.replace('\'', "'\\''")
    );
    let panes_cmd = format!(
        "tmux list-panes -t '{}' -s -F '#{{pane_id}},#{{pane_width}},#{{pane_height}},#{{pane_active}},#{{window_id}},#{{pane_title}}'",
        session_id.replace('\'', "'\\''")
    );

    let (sess_out, _, _) = state.pool.with_session(&host_id, |session| exec::exec(session, &session_cmd))?;
    let (win_out, _, _) = state.pool.with_session(&host_id, |session| exec::exec(session, &windows_cmd))?;
    let (pane_out, _, _) = state.pool.with_session(&host_id, |session| exec::exec(session, &panes_cmd))?;

    let session_name = sess_out.lines().next().and_then(|l| l.split(',').nth(1)).unwrap_or("");

    let windows: Vec<serde_json::Value> = win_out
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let p: Vec<&str> = line.splitn(4, ',').collect();
            serde_json::json!({
                "id": p.first().unwrap_or(&""),
                "name": p.get(1).unwrap_or(&""),
                "active": p.get(2).unwrap_or(&"0") == &"1",
                "panes": p.get(3).unwrap_or(&"0").parse::<u32>().unwrap_or(0),
            })
        })
        .collect();

    let active_window_id = windows.iter().find(|w| w["active"].as_bool().unwrap_or(false)).map(|w| w["id"].as_str().unwrap_or("").to_string());

    let panes: Vec<serde_json::Value> = pane_out
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let p: Vec<&str> = line.splitn(6, ',').collect();
            serde_json::json!({
                "id": p.first().unwrap_or(&""),
                "width": p.get(1).unwrap_or(&"0").parse::<u32>().unwrap_or(0),
                "height": p.get(2).unwrap_or(&"0").parse::<u32>().unwrap_or(0),
                "active": p.get(3).unwrap_or(&"0") == &"1",
                "windowId": p.get(4).unwrap_or(&""),
                "title": p.get(5).unwrap_or(&""),
            })
        })
        .collect();

    let active_pane_id = panes.iter().find(|p| p["active"].as_bool().unwrap_or(false)).map(|p| p["id"].as_str().unwrap_or("").to_string());

    Ok(serde_json::json!({
        "sessionId": session_id,
        "sessionName": session_name,
        "windows": windows,
        "panes": panes,
        "activeWindowId": active_window_id,
        "activePaneId": active_pane_id,
    }))
}

// ── Terminal ──

#[tauri::command]
pub fn attach_terminal(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
    host_id: String,
    session_id: String,
    cols: Option<u32>,
    rows: Option<u32>,
) -> Result<(), String> {
    let host = load_hosts(&state.hosts_config_path)
        .into_iter()
        .find(|h| h.id == host_id)
        .ok_or_else(|| format!("Host {host_id} not found"))?;

    let auth = match host.auth_type.as_str() {
        "key" => crate::ssh::connection::AuthMethod::KeyFile {
            path: host.key_path.unwrap_or_default(),
            passphrase: host.key_passphrase,
        },
        "agent" => crate::ssh::connection::AuthMethod::Agent,
        _ => crate::ssh::connection::AuthMethod::Password {
            password: host.password.unwrap_or_default(),
        },
    };

    let config = crate::ssh::connection::HostConfig {
        id: host.id,
        name: host.name,
        host: host.host,
        port: host.port,
        username: host.username,
        auth,
    };

    state.shell_manager.start(app, &config, session_id, cols.unwrap_or(120), rows.unwrap_or(36))
}

#[tauri::command]
pub fn detach_terminal(state: State<'_, Arc<AppState>>, session_id: String) -> Result<(), String> {
    state.shell_manager.close(&session_id)
}

#[tauri::command]
pub fn send_terminal_input(
    state: State<'_, Arc<AppState>>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    state.shell_manager.write(&session_id, &data)
}

#[tauri::command]
pub fn resize_terminal(
    state: State<'_, Arc<AppState>>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    state.shell_manager.resize(&session_id, cols, rows)
}

// ── Files (SFTP) ──

#[tauri::command]
pub fn list_file_roots(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    let hosts = load_hosts(&state.hosts_config_path);
    let roots: Vec<serde_json::Value> = hosts
        .iter()
        .map(|h| {
            serde_json::json!({
                "id": h.id,
                "name": h.name,
                "host": h.host,
            })
        })
        .collect();
    Ok(serde_json::to_value(roots).unwrap())
}

#[tauri::command]
pub fn list_files(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    root: String,
    path: String,
) -> Result<serde_json::Value, String> {
    let full_path = if path.is_empty() { "/".to_string() } else { path };
    let items = state.pool.with_session(&host_id, |session| {
        crate::ssh::sftp::list_dir(session, &full_path)
    })?;
    Ok(serde_json::json!({
        "root": root,
        "path": full_path,
        "items": items,
    }))
}

#[tauri::command]
pub fn read_file_content(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    root: String,
    path: String,
) -> Result<serde_json::Value, String> {
    let data = state.pool.with_session(&host_id, |session| {
        crate::ssh::sftp::read_file(session, &path)
    })?;
    let content = String::from_utf8_lossy(&data).to_string();
    let is_binary = data.iter().any(|&b| b == 0);
    Ok(serde_json::json!({
        "root": root,
        "path": path,
        "content": if is_binary { "" } else { &content },
        "binary": is_binary,
        "size": data.len(),
    }))
}

#[tauri::command]
pub fn read_file_preview(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    root: String,
    path: String,
    line: Option<u32>,
) -> Result<serde_json::Value, String> {
    let data = state.pool.with_session(&host_id, |session| {
        crate::ssh::sftp::read_file(session, &path)
    })?;
    let content = String::from_utf8_lossy(&data).to_string();
    let lines: Vec<&str> = content.lines().collect();
    let start = (line.unwrap_or(1).saturating_sub(1)) as usize;
    let preview: Vec<&str> = lines[start..std::cmp::min(start + 100, lines.len())].to_vec();
    Ok(serde_json::json!({
        "root": root,
        "path": path,
        "lines": preview,
        "totalLines": lines.len(),
    }))
}

#[tauri::command]
pub fn save_file_content(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    root: String,
    path: String,
    content: String,
    modified_at: Option<String>,
) -> Result<serde_json::Value, String> {
    state.pool.with_session(&host_id, |session| {
        crate::ssh::sftp::write_file(session, &path, content.as_bytes())
    })?;
    Ok(serde_json::json!({
        "ok": true,
        "content": content,
        "size": content.len(),
    }))
}

#[tauri::command]
pub fn search_files_by_name(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    root: String,
    q: String,
    base_path: Option<String>,
) -> Result<serde_json::Value, String> {
    let search_path = base_path.as_deref().unwrap_or("/");
    let cmd = format!("find {} -name '*{}*' -maxdepth 5 2>/dev/null | head -50", search_path, q.replace('\'', "'\\''"));
    let output = state.pool.with_session(&host_id, |session| exec::exec(session, &cmd))?;
    let (stdout, _, _) = output;
    let items: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let is_dir = line.ends_with('/');
            serde_json::json!({
                "name": std::path::Path::new(line).file_name().unwrap_or_default().to_string_lossy(),
                "path": line,
                "isDir": is_dir,
            })
        })
        .collect();
    Ok(serde_json::to_value(items).unwrap())
}

#[tauri::command]
pub fn search_files_by_content(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    root: String,
    q: String,
    base_path: Option<String>,
) -> Result<serde_json::Value, String> {
    let search_path = base_path.as_deref().unwrap_or("/");
    let cmd = format!("grep -rnl '{}' {} 2>/dev/null | head -50", q.replace('\'', "'\\''"), search_path);
    let output = state.pool.with_session(&host_id, |session| exec::exec(session, &cmd))?;
    let (stdout, _, _) = output;
    let items: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            serde_json::json!({
                "path": line,
                "name": std::path::Path::new(line).file_name().unwrap_or_default().to_string_lossy(),
            })
        })
        .collect();
    Ok(serde_json::to_value(items).unwrap())
}

#[tauri::command]
pub fn get_default_upload_target(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    pane_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let path = if let Some(pid) = pane_id {
        let cmd = format!("tmux display-message -t '{}' -p '#{{pane_current_path}}'", pid.replace('\'', "'\\''"));
        let output = state.pool.with_session(&host_id, |session| exec::exec(session, &cmd))?;
        output.0.trim().to_string()
    } else {
        "/tmp".to_string()
    };
    Ok(serde_json::json!({
        "hostId": host_id,
        "path": path,
    }))
}

#[tauri::command]
pub fn upload_file(
    state: State<'_, Arc<AppState>>,
    host_id: String,
    root_id: String,
    target_path: String,
    file_name: String,
    file_data: Vec<u8>,
) -> Result<serde_json::Value, String> {
    let full_path = format!("{}/{}", target_path.trim_end_matches('/'), file_name);
    state.pool.with_session(&host_id, |session| {
        crate::ssh::sftp::write_file(session, &full_path, &file_data)
    })?;
    Ok(serde_json::json!({
        "ok": true,
        "path": full_path,
        "size": file_data.len(),
    }))
}

// ── System ──

#[tauri::command]
pub fn get_system_info(state: State<'_, Arc<AppState>>, host_id: Option<String>) -> Result<serde_json::Value, String> {
    let connected = state.pool.list_connected();
    let hid = host_id.as_deref().or(connected.first().map(|s| s.as_str())).ok_or("No connected host")?;
    let cmd = r#"echo '{"cpu":'$(top -bn1 | grep 'Cpu(s)' | awk '{print $2}')',"mem":{"used":'$(free -m | awk '/Mem:/{print $3}')',"total":'$(free -m | awk '/Mem:/{print $2}')'},"disks":['$(df -h / | awk 'NR==2{printf "{\"mount\":\"%s\",\"used\":%d,\"total\":%d}",$6,substr($3,1,length($3)-1),substr($2,1,length($2)-1)}')']}'"#;
    let output = state.pool.with_session(hid, |session| exec::exec(session, cmd))?;
    let (stdout, _, _) = output;
    serde_json::from_str(stdout.trim()).map_err(|e| format!("Parse system info failed: {e}"))
}

// ── Preferences ──

fn load_preferences(path: &std::path::Path) -> serde_json::Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}))
}

#[tauri::command]
pub fn get_preferences(state: State<'_, Arc<AppState>>, profile: Option<String>) -> Result<serde_json::Value, String> {
    Ok(load_preferences(&state.preferences_path))
}

#[tauri::command]
pub fn update_preferences(
    state: State<'_, Arc<AppState>>,
    profile: Option<String>,
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut prefs = load_preferences(&state.preferences_path);
    if let (Some(obj), Some(patch)) = (prefs.as_object_mut(), payload.as_object()) {
        for (k, v) in patch {
            obj.insert(k.clone(), v.clone());
        }
    }
    let json = serde_json::to_string_pretty(&prefs).map_err(|e| format!("Serialize failed: {e}"))?;
    if let Some(parent) = state.preferences_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Create dir failed: {e}"))?;
    }
    std::fs::write(&state.preferences_path, json).map_err(|e| format!("Write failed: {e}"))?;
    Ok(prefs)
}
