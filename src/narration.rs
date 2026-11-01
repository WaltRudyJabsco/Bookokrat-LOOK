//! Local speech bridge. The reader owns highlighting; LOOK owns voice processes.
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};

pub struct Narrator {
    child: Child,
    input: ChildStdin,
    events: Receiver<Value>,
    pub token: u64,
    pub node: Option<usize>,
    pub chapter: Option<String>,
    pub playing: bool,
    pub status: String,
}

impl Narrator {
    pub fn start() -> Result<Self, String> {
        let helper = std::env::var_os("LOOK_BOOK_SPEECH_HELPER")
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|home| home.join(".local/share/look/book_speech.py")))
            .filter(|path| path.is_file())
            .ok_or("Install LOOK Books speech support to enable narration")?;
        let python =
            std::env::var_os("LOOK_BOOK_SPEECH_PYTHON").unwrap_or_else(|| "python3".into());
        Self::with_helper(&python, &helper)
    }

    fn with_helper(python: &std::ffi::OsStr, helper: &std::path::Path) -> Result<Self, String> {
        let mut child = Command::new(python)
            .arg("-u")
            .arg(helper)
            .arg("--native-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| error.to_string())?;
        let input = child.stdin.take().ok_or("Speech input unavailable")?;
        let output = child.stdout.take().ok_or("Speech output unavailable")?;
        let (send, events) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines().map_while(Result::ok) {
                if let Ok(value) = serde_json::from_str(&line) {
                    if send.send(value).is_err() {
                        return;
                    }
                }
            }
            let _ = send.send(json!({"state":"disconnected"}));
        });
        Ok(Self {
            child,
            input,
            events,
            token: 0,
            node: None,
            chapter: None,
            playing: false,
            status: "Preparing narration…".into(),
        })
    }

    pub fn command(&mut self, action: &str) -> Result<(), String> {
        if action == "stop" {
            self.token += 1;
            self.node = None;
            self.playing = false;
        }
        writeln!(self.input, "{}", json!({"action":action,"id":self.token}))
            .and_then(|_| self.input.flush())
            .map_err(|e| e.to_string())
    }

    pub fn speak(
        &mut self,
        node: usize,
        chapter: Option<String>,
        text: &str,
    ) -> Result<(), String> {
        self.token += 1;
        self.node = Some(node);
        self.chapter = chapter;
        self.playing = true;
        writeln!(
            self.input,
            "{}",
            json!({"action":"speak","id":self.token,"text":text})
        )
        .and_then(|_| self.input.flush())
        .map_err(|e| e.to_string())
    }

    pub fn poll(&mut self) -> Vec<Value> {
        self.events.try_iter().collect()
    }
}

impl Drop for Narrator {
    fn drop(&mut self) {
        let _ = self.command("quit");
        // EOF also retires the helper if the reader is terminated unexpectedly.
        for _ in 0..250 {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bridge_keeps_request_ids_and_retires_its_child() {
        let directory = tempfile::tempdir().unwrap();
        let helper = directory.path().join("fake.py");
        std::fs::write(
            &helper,
            r#"
import json,sys
for line in sys.stdin:
    value=json.loads(line)
    if value['action']=='quit': break
    print(json.dumps(dict(value,state='reading',voice='Fixture',speed=1)),flush=True)
"#,
        )
        .unwrap();
        let mut bridge = Narrator::with_helper(std::ffi::OsStr::new("python3"), &helper).unwrap();
        bridge
            .speak(4, Some("chapter.xhtml".into()), "Literal $(text)")
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut events = Vec::new();
        while events.is_empty() && std::time::Instant::now() < deadline {
            events = bridge.poll();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(events[0]["id"], 1);
        assert_eq!(events[0]["text"], "Literal $(text)");
        bridge.command("stop").unwrap();
        assert_eq!(bridge.token, 2);
        assert!(!bridge.playing);
        assert_eq!(bridge.node, None);
    }
}
