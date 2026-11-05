//! Minimal Discord Rich Presence client over Discord's local IPC
//! (named pipe on Windows, Unix socket on macOS/Linux).
//!
//! Frame format: [opcode: u32 LE][length: u32 LE][JSON payload]

use std::io::{self, Read, Write};

const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;
const OP_PING: u32 = 3;
const OP_PONG: u32 = 4;

pub struct Activity {
    pub details: String,
    pub state: String,
    pub start_ms: u64,
    pub large_text: String,
}

trait Stream: Read + Write + Send {}
impl<T: Read + Write + Send> Stream for T {}

pub struct Discord {
    client_id: &'static str,
    conn: Option<Box<dyn Stream>>,
    nonce: u64,
}

impl Discord {
    pub fn new(client_id: &'static str) -> Self {
        Self { client_id, conn: None, nonce: 0 }
    }

    pub fn is_connected(&self) -> bool {
        self.conn.is_some()
    }

    /// Try each Discord IPC endpoint and perform the handshake.
    pub fn connect(&mut self) -> io::Result<()> {
        let mut last_err = io::Error::new(io::ErrorKind::NotFound, "Discord is not running");
        for mut stream in open_endpoints() {
            match handshake(&mut *stream, self.client_id) {
                Ok(()) => {
                    self.conn = Some(stream);
                    return Ok(());
                }
                Err(e) => last_err = e,
            }
        }
        Err(last_err)
    }

    /// Set (Some) or clear (None) the activity. Drops the connection on error.
    pub fn set_activity(&mut self, activity: Option<&Activity>) -> io::Result<()> {
        self.nonce += 1;
        let activity_json = match activity {
            Some(a) => format!(
                r#"{{"details":{},"state":{},"timestamps":{{"start":{}}},"assets":{{"large_image":"figma","large_text":{}}},"instance":false}}"#,
                json_str(&a.details),
                json_str(&a.state),
                a.start_ms,
                json_str(&a.large_text),
            ),
            None => "null".into(),
        };
        let payload = format!(
            r#"{{"cmd":"SET_ACTIVITY","args":{{"pid":{},"activity":{}}},"nonce":"{}"}}"#,
            std::process::id(),
            activity_json,
            self.nonce,
        );

        let result = self.request(&payload);
        if result.is_err() {
            self.conn = None;
        }
        result
    }

    fn request(&mut self, payload: &str) -> io::Result<()> {
        let stream = self.conn.as_mut().ok_or(io::ErrorKind::NotConnected)?;
        write_frame(&mut **stream, OP_FRAME, payload)?;
        loop {
            let (op, reply) = read_frame(&mut **stream)?;
            match op {
                OP_PING => write_frame(&mut **stream, OP_PONG, &reply)?,
                OP_CLOSE => return Err(io::Error::new(io::ErrorKind::ConnectionAborted, reply)),
                _ => {
                    if reply.contains(r#""evt":"ERROR""#) {
                        eprintln!("[discord] error: {reply}");
                    }
                    return Ok(());
                }
            }
        }
    }
}

fn handshake(stream: &mut dyn Stream, client_id: &str) -> io::Result<()> {
    write_frame(stream, OP_HANDSHAKE, &format!(r#"{{"v":1,"client_id":{}}}"#, json_str(client_id)))?;
    let (op, reply) = read_frame(stream)?;
    if op == OP_FRAME && reply.contains("READY") {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::ConnectionRefused, reply))
    }
}

fn write_frame(stream: &mut dyn Stream, op: u32, payload: &str) -> io::Result<()> {
    let mut buf = Vec::with_capacity(8 + payload.len());
    buf.extend_from_slice(&op.to_le_bytes());
    buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    buf.extend_from_slice(payload.as_bytes());
    stream.write_all(&buf)?;
    stream.flush()
}

fn read_frame(stream: &mut dyn Stream) -> io::Result<(u32, String)> {
    let mut header = [0u8; 8];
    stream.read_exact(&mut header)?;
    let op = u32::from_le_bytes(header[0..4].try_into().unwrap());
    let len = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
    if len > 1 << 20 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "frame too large"));
    }
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload)?;
    Ok((op, String::from_utf8_lossy(&payload).into_owned()))
}

/// JSON string literal with escaping.
fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(windows)]
fn open_endpoints() -> impl Iterator<Item = Box<dyn Stream>> {
    (0..10).filter_map(|i| {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!(r"\\.\pipe\discord-ipc-{i}"))
            .ok()
            .map(|f| Box::new(f) as Box<dyn Stream>)
    })
}

#[cfg(unix)]
fn open_endpoints() -> impl Iterator<Item = Box<dyn Stream>> {
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    let mut dirs: Vec<String> = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .collect();
    dirs.push("/tmp".into());

    dirs.into_iter()
        .flat_map(|dir| (0..10).map(move |i| format!("{}/discord-ipc-{i}", dir.trim_end_matches('/'))))
        .filter_map(|path| UnixStream::connect(path).ok())
        .map(|s| {
            let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
            let _ = s.set_write_timeout(Some(Duration::from_secs(5)));
            Box::new(s) as Box<dyn Stream>
        })
}

#[cfg(test)]
mod tests {
    use super::json_str;

    #[test]
    fn escapes_json() {
        assert_eq!(json_str(r#"a "b" \c"#), r#""a \"b\" \\c""#);
        assert_eq!(json_str("line\nbreak"), r#""line\nbreak""#);
        assert_eq!(json_str("Café – ✨"), "\"Café – ✨\"");
    }
}
