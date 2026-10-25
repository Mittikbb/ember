//! Discord's local RPC pipe, used only to set the activity. No OAuth and no
//! secret: SET_ACTIVITY needs just the client id of the user's Discord app,
//! whose name becomes the "Playing …" line.
//!
//! Every client gets its own connection and its own activity, so this runs
//! side by side with Lumen's connection (its "Listening" status) or a game's.

use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use windows::Win32::{Foundation::HANDLE, System::Pipes::PeekNamedPipe};

use crate::presence::Activity;

const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;
const OP_PING: u32 = 3;
const OP_PONG: u32 = 4;
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

/// Messages are codes the window translates; Discord's own errors pass through as "Discord: …".
pub const NOT_RUNNING: &str = "discord.notRunning";

pub struct Ipc {
    file: File,
    nonce: u64,
    /// The Discord account, from the READY event.
    pub user: Option<String>,
}

impl Ipc {
    pub fn connect(client_id: &str) -> Result<Ipc, String> {
        let mut denied = false;
        for i in 0..10 {
            match OpenOptions::new().read(true).write(true).open(format!(r"\\.\pipe\discord-ipc-{i}")) {
                Ok(file) => {
                    let mut ipc = Ipc { file, nonce: 0, user: None };
                    ipc.send(OP_HANDSHAKE, &json!({ "v": 1, "client_id": client_id.trim() }))?;
                    let ready = ipc.wait(|m| m["evt"] == "READY")?;
                    let user = &ready["data"]["user"];
                    ipc.user = user["global_name"]
                        .as_str()
                        .or(user["username"].as_str())
                        .map(String::from);
                    return Ok(ipc);
                }
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => denied = true,
                Err(_) => {}
            }
        }
        Err(if denied {
            "discord.admin".into()
        } else {
            NOT_RUNNING.into()
        })
    }

    /// Sets the activity, or clears it with `None`.
    pub fn set_activity(&mut self, activity: Option<&Activity>) -> Result<(), String> {
        let mut args = json!({ "pid": std::process::id() });
        if let Some(a) = activity {
            args["activity"] = serde_json::to_value(a).map_err(|e| e.to_string())?;
        }
        self.nonce += 1;
        let nonce = self.nonce.to_string();
        self.send(OP_FRAME, &json!({ "cmd": "SET_ACTIVITY", "args": args, "nonce": nonce }))?;
        let reply = self.wait(|m| m["nonce"] == nonce.as_str())?;
        if reply["evt"] == "ERROR" {
            let message = reply["data"]["message"].as_str().unwrap_or("error");
            return Err(format!("Discord: {message}"));
        }
        Ok(())
    }

    /// Answers pings that came in between updates.
    pub fn idle(&mut self) -> Result<(), String> {
        while self.pending()? {
            self.read_frame()?;
        }
        Ok(())
    }

    fn send(&mut self, op: u32, body: &Value) -> Result<(), String> {
        let body = body.to_string();
        let mut frame = Vec::with_capacity(8 + body.len());
        frame.extend_from_slice(&op.to_le_bytes());
        frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
        frame.extend_from_slice(body.as_bytes());
        self.file.write_all(&frame).map_err(closed)
    }

    fn wait(&mut self, matches: impl Fn(&Value) -> bool) -> Result<Value, String> {
        let deadline = Instant::now() + REPLY_TIMEOUT;
        loop {
            if self.pending()? {
                if let Some(message) = self.read_frame()? {
                    if matches(&message) {
                        return Ok(message);
                    }
                }
            } else if Instant::now() > deadline {
                return Err("discord.noReply".into());
            } else {
                thread::sleep(Duration::from_millis(20));
            }
        }
    }

    /// One frame; pings are answered and give `None`.
    fn read_frame(&mut self) -> Result<Option<Value>, String> {
        let mut header = [0u8; 8];
        self.file.read_exact(&mut header).map_err(closed)?;
        let op = u32::from_le_bytes(header[..4].try_into().unwrap());
        let len = u32::from_le_bytes(header[4..].try_into().unwrap()) as usize;
        let mut body = vec![0u8; len];
        self.file.read_exact(&mut body).map_err(closed)?;
        let value: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
        match op {
            OP_FRAME => Ok(Some(value)),
            OP_CLOSE => {
                let reason = value["message"].as_str().unwrap_or("connection closed");
                Err(format!("Discord: {reason}"))
            }
            OP_PING => {
                self.send(OP_PONG, &value)?;
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    #[cfg(windows)]
    fn pending(&self) -> Result<bool, String> {
        let mut available = 0u32;
        unsafe { PeekNamedPipe(HANDLE(self.file.as_raw_handle()), None, 0, None, Some(&mut available), None) }
            .map_err(|_| "discord.closed".to_string())?;
        Ok(available > 0)
    }

    #[cfg(not(windows))]
    fn pending(&self) -> Result<bool, String> {
        Err(NOT_RUNNING.into())
    }
}

fn closed(_: std::io::Error) -> String {
    "discord.closed".into()
}
