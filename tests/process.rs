//! Drives the real `tezbar-base64` executable over stdio JSONL pipes. The test
//! plays the Tezbar host: it answers typed `host.*` reverse RPCs from a script
//! and records them. No real clipboard, paste, browser or window is touched.
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
};

const BIN: &str = env!("CARGO_BIN_EXE_tezbar-base64");
const ID: &str = "com.tezbar.native.base64";
const GRANTS: &[&str] = &[
    "clipboard.read",
    "clipboard.write",
    "clipboard.pasteText",
    "shell.open",
    "window.visibility",
];

type Reply = Box<dyn FnMut(&str, &Value) -> Result<Value, String>>;

struct Peer {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    calls: Vec<(String, Value)>,
    reply: Reply,
    sequence: u64,
    _dir: tempfile::TempDir,
}

fn clipboard(text: &'static str) -> Reply {
    Box::new(move |method, _| match method {
        "host.clipboard.read" => Ok(json!(text)),
        _ => Ok(Value::Null),
    })
}

impl Peer {
    async fn start(grants: &[&str], reply: Reply) -> Self {
        let mut child = Command::new(BIN)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().unwrap());
        let dir = tempfile::tempdir().unwrap();
        let mut peer = Self {
            child,
            input,
            output,
            calls: vec![],
            reply,
            sequence: 1,
            _dir: dir,
        };
        let hello = peer.frame().await;
        assert_eq!(hello["type"], "hello");
        assert_eq!(hello["id"], ID);
        assert_eq!(hello["protocol"], 1);
        assert_eq!(hello["version"], env!("CARGO_PKG_VERSION"));
        let root = peer._dir.path().to_path_buf();
        let init = peer
            .request(
                "initialize",
                json!({"protocol":1,"extensionId":ID,
                    "platform":if cfg!(windows) {"Windows"} else {"macOS"},
                    "packageDir":env!("CARGO_MANIFEST_DIR"),
                    "dataDir":root.join("data"),"tempDir":root.join("temp"),
                    "grantedCapabilities":grants,"reason":"command","migrationPaths":{}}),
            )
            .await
            .unwrap();
        assert_eq!(init["ready"], true);
        peer
    }
    async fn send(&mut self, value: Value) {
        let mut bytes = serde_json::to_vec(&value).unwrap();
        bytes.push(b'\n');
        self.input
            .as_mut()
            .unwrap()
            .write_all(&bytes)
            .await
            .unwrap();
    }
    async fn frame(&mut self) -> Value {
        let mut line = String::new();
        let n = tokio::time::timeout(Duration::from_secs(10), self.output.read_line(&mut line))
            .await
            .expect("protocol timeout")
            .unwrap();
        assert!(n > 0, "unexpected EOF");
        assert!(line.ends_with('\n'), "LF framing");
        serde_json::from_str(&line).unwrap()
    }
    /// Sends a host request and services reverse RPCs until its response.
    async fn request(&mut self, method: &str, params: Value) -> Result<Value, Value> {
        let id = format!("h:00000000-0000-4000-8000-{:012x}", self.sequence);
        self.sequence += 1;
        self.send(json!({"type":"request","id":id,"method":method,"params":params}))
            .await;
        loop {
            let frame = self.frame().await;
            match frame["type"].as_str() {
                Some("response") if frame["id"] == id.as_str() => {
                    return match frame.get("error") {
                        Some(error) => Err(error.clone()),
                        None => Ok(frame["result"].clone()),
                    };
                }
                Some("request") => {
                    let method = frame["method"].as_str().unwrap().to_owned();
                    assert!(frame["id"].as_str().unwrap().starts_with("e:"));
                    assert!(method.starts_with("host."), "non-typed RPC {method}");
                    let params = frame["params"].clone();
                    let result = (self.reply)(&method, &params);
                    self.calls.push((method, params));
                    let reply = match result {
                        Ok(value) => json!({"type":"response","id":frame["id"],"result":value}),
                        Err(message) => json!({"type":"response","id":frame["id"],
                            "error":{"code":"host_error","message":message}}),
                    };
                    self.send(reply).await;
                }
                Some("event") => {}
                other => panic!("unexpected frame {other:?}: {frame}"),
            }
        }
    }
    async fn open(&mut self, session: &str, command: &str, prefs: Value) -> Result<Value, Value> {
        self.request(
            "session.open",
            json!({"sessionId":session,"commandName":command,"preferences":prefs,"argumentValues":{}}),
        )
        .await
    }
    async fn call(
        &mut self,
        session: &str,
        operation: &str,
        payload: Value,
    ) -> Result<Value, Value> {
        self.request(
            "session.call",
            json!({"sessionId":session,"operation":operation,"payload":payload}),
        )
        .await
    }
    fn methods(&self) -> Vec<&str> {
        self.calls.iter().map(|(m, _)| m.as_str()).collect()
    }
    async fn shutdown(mut self) {
        self.request("shutdown", json!({})).await.unwrap();
        let status = tokio::time::timeout(Duration::from_secs(5), self.child.wait())
            .await
            .unwrap()
            .unwrap();
        assert!(status.success());
    }
}

fn rows(view: &Value) -> Vec<Value> {
    view["root"]["children"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["type"] == "List.Section")
        .flat_map(|s| s["children"].as_array().unwrap().clone())
        .collect()
}

fn action(view: &Value, row: &str, kind: &str) -> String {
    let row = rows(view)
        .into_iter()
        .find(|r| r["props"]["id"] == row)
        .unwrap_or_else(|| panic!("row {row} in {view}"));
    let ids: Vec<&str> = row["props"]["actionIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    view["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["kind"] == kind && ids.contains(&a["id"].as_str().unwrap()))
        .unwrap_or_else(|| panic!("{kind} action on {row}"))["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn message(error: &Value) -> &str {
    error["message"].as_str().unwrap()
}

#[tokio::test]
async fn protocol_info_reports_identity_and_commands() {
    let output = Command::new(BIN)
        .arg("--protocol-info")
        .output()
        .await
        .unwrap();
    assert!(output.status.success());
    let info: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(info["protocol"], 1);
    assert_eq!(info["id"], ID);
    assert_eq!(info["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(info["commands"], json!(["index", "encode", "decode"]));
    let manifest: Value = serde_json::from_str(include_str!("../extension.json")).unwrap();
    assert_eq!(manifest["version"], info["version"]);
    assert_eq!(manifest["id"], ID);
    let names: Vec<&str> = manifest["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["index", "encode", "decode"]);
}

#[tokio::test]
async fn interactive_session_search_actions_and_stale_rejection() {
    let mut peer = Peer::start(GRANTS, clipboard("aGk=")).await;
    let view = peer.open("s1", "index", json!({})).await.unwrap();
    assert_eq!(view["mode"], "view");
    assert_eq!(view["root"]["type"], "List");
    assert_eq!(view["root"]["props"]["__hasServerSearch"], true);
    assert_eq!(view["root"]["props"]["searchText"], "aGk=");
    let r = rows(&view);
    assert_eq!(r[0]["props"]["subtitle"], "YUdrPQ==");
    assert_eq!(r[1]["props"]["subtitle"], "hi");
    assert_eq!(peer.methods(), ["host.clipboard.read"]);
    // Public action metadata never carries the value itself.
    assert!(!view["actions"].to_string().contains("YUdrPQ=="));

    let stale = action(&view, "decode", "copy");
    let url_b64 = "aHR0cHM6Ly9leGFtcGxlLmNvbS9hP2I9Yw==";
    let view = peer
        .call(
            "s1",
            "extension:search-text-changed",
            json!({"searchText":url_b64}),
        )
        .await
        .unwrap();
    assert_eq!(
        rows(&view)[1]["props"]["subtitle"],
        "https://example.com/a?b=c"
    );
    assert_eq!(peer.methods().len(), 1, "search must not reread clipboard");

    let error = peer
        .call("s1", "extension:invoke-action", json!({"actionId":stale}))
        .await
        .unwrap_err();
    assert_eq!(message(&error), "Unknown or stale Base64 action");
    let error = peer
        .call(
            "s1",
            "extension:invoke-action",
            json!({"actionId":"base64:9:decode:openInBrowser","value":"https://evil"}),
        )
        .await
        .unwrap_err();
    assert_eq!(message(&error), "Unknown or stale Base64 action");
    assert_eq!(peer.methods().len(), 1);

    // Refresh keeps action IDs valid.
    let refreshed = peer
        .call("s1", "extension:refresh-session", json!({}))
        .await
        .unwrap();
    assert_eq!(refreshed["actions"], view["actions"]);
    let unchanged = peer
        .call("s1", "extension:load-more", json!({}))
        .await
        .unwrap();
    assert_eq!(unchanged["mode"], "unchanged");

    let copy = action(&view, "decode", "copy");
    let after = peer
        .call("s1", "extension:invoke-action", json!({"actionId":copy}))
        .await
        .unwrap();
    assert_eq!(after["mode"], "view", "copy keeps the view");
    assert_eq!(
        peer.calls[1],
        (
            "host.clipboard.write".into(),
            json!({"text":"https://example.com/a?b=c"})
        )
    );

    let paste = action(&view, "encode", "paste");
    let result = peer
        .call("s1", "extension:invoke-action", json!({"actionId":paste}))
        .await
        .unwrap();
    assert_eq!(result["mode"], "no-view");
    assert_eq!(
        peer.calls[2],
        (
            "host.clipboard.pasteText".into(),
            json!({"text":"YUhSMGNITTZMeTlsZUdGdGNHeGxMbU52YlM5aFAySTlZdz09"})
        )
    );

    let open = action(&view, "decode", "open");
    let result = peer
        .call("s1", "extension:invoke-action", json!({"actionId":open}))
        .await
        .unwrap();
    assert_eq!(result["mode"], "no-view");
    assert_eq!(
        &peer.methods()[3..],
        ["host.shell.open", "host.window.visibility"]
    );
    assert_eq!(
        peer.calls[3].1,
        json!({"target":"https://example.com/a?b=c"})
    );
    assert_eq!(peer.calls[4].1, json!({"visible":false}));

    peer.request("session.dispose", json!({"sessionId":"s1"}))
        .await
        .unwrap();
    assert!(peer
        .call("s1", "extension:refresh-session", json!({}))
        .await
        .is_err());
    peer.shutdown().await;
}

#[tokio::test]
async fn default_action_preference_orders_actions_and_encode_has_no_open() {
    let mut peer = Peer::start(GRANTS, clipboard("https://example.com")).await;
    let view = peer
        .open("s", "index", json!({"defaultAction":"openInBrowser"}))
        .await
        .unwrap();
    let r = rows(&view);
    assert_eq!(
        r.len(),
        1,
        "a URL is not valid Base64, only Encode is shown"
    );
    let ids = r[0]["props"]["actionIds"].as_array().unwrap();
    // Encoded text is not a URL so the preferred open action is omitted.
    assert_eq!(ids.len(), 2);
    assert!(ids[0].as_str().unwrap().ends_with(":copyToClipboard"));
    let view = peer
        .open("p", "index", json!({"defaultAction":"pasteInApp"}))
        .await
        .unwrap();
    let ids = rows(&view)[0]["props"]["actionIds"].clone();
    assert!(ids[0].as_str().unwrap().ends_with(":pasteInApp"));
    assert!(ids[1].as_str().unwrap().ends_with(":copyToClipboard"));
    peer.shutdown().await;
}

#[tokio::test]
async fn clipboard_read_failure_opens_empty_view() {
    let reply: Reply = Box::new(|method, _| match method {
        "host.clipboard.read" => Err("Clipboard access denied".into()),
        _ => Ok(Value::Null),
    });
    let mut peer = Peer::start(GRANTS, reply).await;
    let view = peer.open("s", "index", json!({})).await.unwrap();
    let children = view["root"]["children"].as_array().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["type"], "List.EmptyView");
    assert_eq!(children[0]["props"]["title"], "Nothing to Encode or Decode");
    peer.shutdown().await;
}

#[tokio::test]
async fn encode_and_decode_no_view_commands() {
    let mut peer = Peer::start(GRANTS, clipboard("Café ☕")).await;
    let result = peer.open("e", "encode", json!({})).await.unwrap();
    assert_eq!(
        result,
        json!({"ok":true,"mode":"no-view","message":"Encoded text copied to clipboard"})
    );
    assert_eq!(
        peer.calls,
        [
            ("host.clipboard.read".into(), json!({})),
            (
                "host.clipboard.write".into(),
                json!({"text":"Q2Fmw6kg4piV"})
            ),
            ("host.window.visibility".into(), json!({"visible":false})),
        ]
    );
    peer.request("session.dispose", json!({"sessionId":"e"}))
        .await
        .unwrap();
    // Invalid Base64 (from V1 semantics) fails before any write.
    let error = peer.open("d", "decode", json!({})).await.unwrap_err();
    assert_eq!(message(&error), "The clipboard is not valid Base64 text.");
    assert_eq!(peer.methods().len(), 4);
    assert_eq!(peer.methods()[3], "host.clipboard.read");
    peer.shutdown().await;

    let mut peer = Peer::start(GRANTS, clipboard(" Q2Fm\nw6kg4piV ")).await;
    let result = peer.open("d", "decode", json!({})).await.unwrap();
    assert_eq!(result["message"], "Decoded text copied to clipboard");
    assert_eq!(
        peer.calls[1],
        ("host.clipboard.write".into(), json!({"text":"Café ☕"}))
    );
    assert_eq!(peer.methods()[2], "host.window.visibility");
    peer.shutdown().await;
}

#[tokio::test]
async fn empty_clipboard_and_host_failures_propagate() {
    let mut peer = Peer::start(GRANTS, clipboard("")).await;
    let error = peer.open("e", "encode", json!({})).await.unwrap_err();
    assert_eq!(message(&error), "The clipboard does not contain text.");
    assert_eq!(peer.methods(), ["host.clipboard.read"]);
    peer.shutdown().await;

    let reply: Reply = Box::new(|method, _| match method {
        "host.clipboard.read" => Ok(json!("abc")),
        "host.clipboard.write" => Err("write denied".into()),
        _ => Ok(Value::Null),
    });
    let mut peer = Peer::start(GRANTS, reply).await;
    let error = peer.open("e", "encode", json!({})).await.unwrap_err();
    assert!(message(&error).contains("write denied"), "{error}");
    assert_eq!(
        peer.methods(),
        ["host.clipboard.read", "host.clipboard.write"],
        "no hide after failed copy"
    );
    peer.shutdown().await;
}

#[tokio::test]
async fn ungranted_capability_is_rejected_without_reverse_rpc() {
    let mut peer = Peer::start(&["clipboard.read"], clipboard("abc")).await;
    let error = peer.open("e", "encode", json!({})).await.unwrap_err();
    assert!(message(&error).contains("not granted"), "{error}");
    assert_eq!(peer.methods(), ["host.clipboard.read"]);
    peer.shutdown().await;
}

#[tokio::test]
async fn unknown_command_and_eof_exit() {
    let mut peer = Peer::start(GRANTS, clipboard("")).await;
    assert!(peer.open("x", "nope", json!({})).await.is_err());
    peer.input.take();
    let status = tokio::time::timeout(Duration::from_secs(5), peer.child.wait())
        .await
        .unwrap()
        .unwrap();
    assert!(status.success());
}
