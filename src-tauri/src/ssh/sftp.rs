use ssh2::Session;
use std::io::Read;

pub fn list_dir(session: &Session, path: &str) -> Result<Vec<serde_json::Value>, String> {
    let sftp = session.sftp().map_err(|e| format!("SFTP init failed: {e}"))?;
    let entries = sftp.readdir(std::path::Path::new(path)).map_err(|e| format!("Readdir failed: {e}"))?;

    let items: Vec<serde_json::Value> = entries
        .iter()
        .filter_map(|(path_buf, stat)| {
            let name = path_buf.file_name()?.to_string_lossy().to_string();
            if name.starts_with('.') {
                return None;
            }
            Some(serde_json::json!({
                "name": name,
                "path": path_buf.to_string_lossy(),
                "isDir": stat.is_dir(),
                "size": stat.size.unwrap_or(0),
                "modified": stat.mtime.map(|t| t as i64),
            }))
        })
        .collect();

    Ok(items)
}

pub fn read_file(session: &Session, path: &str) -> Result<Vec<u8>, String> {
    let sftp = session.sftp().map_err(|e| format!("SFTP init failed: {e}"))?;
    let mut file = sftp.open(std::path::Path::new(path)).map_err(|e| format!("Open failed: {e}"))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(|e| format!("Read failed: {e}"))?;
    Ok(buf)
}

pub fn write_file(session: &Session, path: &str, content: &[u8]) -> Result<(), String> {
    let sftp = session.sftp().map_err(|e| format!("SFTP init failed: {e}"))?;
    let mut file = sftp
        .create(std::path::Path::new(path))
        .map_err(|e| format!("Create failed: {e}"))?;
    use std::io::Write;
    file.write_all(content).map_err(|e| format!("Write failed: {e}"))?;
    Ok(())
}

pub fn stat(session: &Session, path: &str) -> Result<serde_json::Value, String> {
    let sftp = session.sftp().map_err(|e| format!("SFTP init failed: {e}"))?;
    let stat = sftp.stat(std::path::Path::new(path)).map_err(|e| format!("Stat failed: {e}"))?;
    Ok(serde_json::json!({
        "size": stat.size.unwrap_or(0),
        "isDir": stat.is_dir(),
        "modified": stat.mtime.map(|t| t as i64),
    }))
}

pub fn mkdir(session: &Session, path: &str) -> Result<(), String> {
    let sftp = session.sftp().map_err(|e| format!("SFTP init failed: {e}"))?;
    sftp.mkdir(std::path::Path::new(path), 0o755).map_err(|e| format!("Mkdir failed: {e}"))?;
    Ok(())
}

pub fn remove(session: &Session, path: &str) -> Result<(), String> {
    let sftp = session.sftp().map_err(|e| format!("SFTP init failed: {e}"))?;
    let p = std::path::Path::new(path);
    let stat = sftp.stat(p).map_err(|e| format!("Stat failed: {e}"))?;
    if stat.is_dir() {
        sftp.rmdir(p).map_err(|e| format!("Rmdir failed: {e}"))?;
    } else {
        sftp.unlink(p).map_err(|e| format!("Unlink failed: {e}"))?;
    }
    Ok(())
}

pub fn rename(session: &Session, from: &str, to: &str) -> Result<(), String> {
    let sftp = session.sftp().map_err(|e| format!("SFTP init failed: {e}"))?;
    sftp.rename(std::path::Path::new(from), std::path::Path::new(to), None)
        .map_err(|e| format!("Rename failed: {e}"))?;
    Ok(())
}
