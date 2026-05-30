use ssh2::Session;
use std::io::Read;

pub fn exec(session: &Session, command: &str) -> Result<(String, String, i32), String> {
    let mut channel = session.channel_session().map_err(|e| format!("Channel open failed: {e}"))?;
    channel.exec(command).map_err(|e| format!("Exec failed: {e}"))?;

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    channel
        .read_to_end(&mut stdout)
        .map_err(|e| format!("Read stdout failed: {e}"))?;
    channel
        .stderr()
        .read_to_end(&mut stderr)
        .map_err(|e| format!("Read stderr failed: {e}"))?;

    channel.wait_close().map_err(|e| format!("Wait close failed: {e}"))?;
    let exit_code = channel.exit_status().unwrap_or(-1);

    Ok((
        String::from_utf8_lossy(&stdout).to_string(),
        String::from_utf8_lossy(&stderr).to_string(),
        exit_code,
    ))
}

pub fn exec_json<T: serde::de::DeserializeOwned>(session: &Session, command: &str) -> Result<T, String> {
    let (stdout, stderr, exit_code) = exec(session, command)?;
    if exit_code != 0 {
        return Err(format!("Command failed (exit {exit_code}): {stderr}"));
    }
    serde_json::from_str(&stdout).map_err(|e| format!("JSON parse failed: {e}, output: {stdout}"))
}
