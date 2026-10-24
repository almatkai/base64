//! Rust translation of the MIT Raycast/Tezbar Base64 extension. See LICENSE.
//! Compiled-in library, not an executable or dynamic extension loader.
use base64::{engine::general_purpose, Engine};
use serde_json::{json, Map, Value};
use std::{future::Future, pin::Pin};

pub const ICON: &[u8] = include_bytes!("../assets/command-icon.png");
pub const MANIFEST: &str = include_str!("../manifest.json");
pub type Result = std::result::Result<Value, String>;
pub type HostFuture<'a> = Pin<Box<dyn Future<Output = Result> + Send + 'a>>;

/// The host owns OS effects. Route names/argument shapes match CoreContext::request.
pub trait Host: Sync {
    fn request<'a>(&'a self, route: &'a str, args: Vec<Value>) -> HostFuture<'a>;
}

pub fn commands() -> &'static [&'static str] {
    &["index", "encode", "decode"]
}

pub fn encode_base64(text: &str) -> String {
    general_purpose::STANDARD.encode(text.as_bytes())
}

// ECMAScript \s, not Rust's Unicode White_Space (which also includes U+0085).
fn js_whitespace(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' |
        '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' |
        '\u{205f}' | '\u{3000}' | '\u{feff}')
}

fn decode_bytes(text: &str) -> std::result::Result<Vec<u8>, String> {
    let compact: String = text.chars().filter(|c| !js_whitespace(*c)).collect();
    let unpadded = compact.trim_end_matches('=');
    if compact.is_empty()
        || compact.len() % 4 == 1
        || compact.len() - unpadded.len() > 2
        || !unpadded
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'+' || c == b'/')
    {
        return Err("Input is not valid Base64 text.".into());
    }
    // NO_PAD rejects non-zero trailing bits. V1 compares a canonical re-encode
    // after removing padding, hence it accepts e.g. Zg= and even ==, but not Zm8==.
    general_purpose::STANDARD_NO_PAD
        .decode(unpadded)
        .map_err(|_| "Input is not valid Base64 text.".into())
}

pub fn is_valid_base64(text: &str) -> bool {
    decode_bytes(text).is_ok()
}

pub fn decode_base64(text: &str) -> std::result::Result<String, String> {
    // Matches Buffer.toString('utf8'): invalid UTF-8 is replaced, not rejected.
    decode_bytes(text).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

fn preview(text: &str) -> String {
    text.replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn is_http_url(text: &str) -> bool {
    url::Url::parse(text).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

fn unchanged() -> Value {
    json!({"ok":true,"mode":"unchanged"})
}

fn no_view(message: &str) -> Value {
    json!({"ok":true,"mode":"no-view","message":message})
}

fn next_revision(state: &Value) -> std::result::Result<u64, String> {
    state["revision"]
        .as_u64()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| "Base64 session revision exhausted".into())
}

fn update_input(state: &mut Value, input: &str) -> std::result::Result<(), String> {
    state["revision"] = json!(next_revision(state)?);
    state["input"] = json!(input);
    Ok(())
}

fn view(state: &mut Value) -> Value {
    let input = state["input"].as_str().unwrap_or("").to_owned();
    let revision = state["revision"].as_u64().unwrap_or(0);
    let preferred = state["defaultAction"].as_str().unwrap_or("copyToClipboard");
    let mut verbs = vec!["copyToClipboard", "pasteInApp", "openInBrowser"];
    if verbs.contains(&preferred) {
        // Preserve the original relative order of the remaining actions.
        verbs = std::iter::once(preferred)
            .chain(
                ["copyToClipboard", "pasteInApp", "openInBrowser"]
                    .into_iter()
                    .filter(|v| *v != preferred),
            )
            .collect();
    }
    let mut results = Vec::new();
    if !input.is_empty() {
        results.push((
            "encode",
            "Encode",
            encode_base64(&input),
            "Icon.CodeBlock",
            "blue",
        ));
        if let Ok(decoded) = decode_base64(&input) {
            results.push(("decode", "Decode", decoded, "Icon.Code", "purple"));
        }
    }
    let mut actions = Vec::new();
    let mut targets = Map::new();
    let mut rows = Vec::new();
    for (key, title, value, icon, color) in results {
        let mut ids = Vec::new();
        for verb in &verbs {
            let (title, kind) = match *verb {
                "copyToClipboard" => ("Copy to Clipboard", "copy"),
                "pasteInApp" => ("Paste in Active App", "paste"),
                "openInBrowser" if is_http_url(&value) => ("Open URL in Browser", "open"),
                _ => continue,
            };
            let id = format!("base64:{revision}:{key}:{verb}");
            // Never put content/url in public action metadata: the host must call
            // invoke-action, which resolves the current authoritative target.
            actions.push(json!({"id":id,"title":title,"kind":kind}));
            targets.insert(id.clone(), json!({"verb":verb,"value":value}));
            ids.push(id);
        }
        rows.push(json!({"type":"List.Item","props":{"id":key,"title":title,"subtitle":preview(&value),"icon":{"source":icon,"tintColor":color},"actionIds":ids}}));
    }
    state["targets"] = Value::Object(targets);
    let mut children = Vec::new();
    if !input.is_empty() {
        children.push(json!({"type":"List.Section","props":{"title":format!("Input: {}", preview(&input))},"children":rows}));
    }
    children.push(json!({"type":"List.EmptyView","props":{"icon":{"source":"Icon.QuestionMarkCircle","tintColor":"secondaryText"},"title":"Nothing to Encode or Decode","description":"Copy text to the clipboard or type into the search field."}}));
    json!({"ok":true,"mode":"view","title":"Base64","root":{"type":"List","props":{"searchText":input,"searchBarPlaceholder":"Text to encode or decode...","__hasServerSearch":true},"children":children},"actions":actions})
}

async fn effect(host: &impl Host, route: &str, args: Vec<Value>) -> Result {
    let value = host.request(route, args).await?;
    // Most core routes return Err; preserve explicit failure envelopes too.
    if value["ok"] == false {
        return Err(value["error"]
            .as_str()
            .or_else(|| value["message"].as_str())
            .unwrap_or("Host operation failed")
            .to_owned());
    }
    Ok(value)
}

async fn clipboard(host: &impl Host) -> std::result::Result<String, String> {
    effect(host, "clipboard:read", vec![])
        .await?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "Clipboard did not return text.".into())
}

/// Tezbar's actual compiled adapter contract, with an injected host for portability.
pub async fn handle(
    host: &impl Host,
    command: &str,
    operation: &str,
    payload: &Value,
    state: &mut Value,
) -> Result {
    if !commands().contains(&command) {
        return Err("Unknown Base64 command".into());
    }
    if operation == "extension:dispose-session" {
        let revision = next_revision(state)?;
        *state = json!({"revision":revision});
        return Ok(unchanged());
    }
    match (command, operation) {
        ("index", "extension:run-command") => {
            let revision = next_revision(state)?;
            // Interactive clipboard failure intentionally leaves the original empty UX.
            let input = clipboard(host).await.unwrap_or_default();
            *state = json!({"active":true,"revision":revision,"input":input,"defaultAction":payload["preferences"]["defaultAction"].as_str().unwrap_or("copyToClipboard")});
            Ok(view(state))
        }
        ("encode" | "decode", "extension:run-command") => {
            let text = clipboard(host).await?;
            let (result, message) = if command == "encode" {
                if text.is_empty() {
                    return Err("The clipboard does not contain text.".into());
                }
                (encode_base64(&text), "Encoded text copied to clipboard")
            } else {
                if text.is_empty() || !is_valid_base64(&text) {
                    return Err("The clipboard is not valid Base64 text.".into());
                }
                (decode_base64(&text)?, "Decoded text copied to clipboard")
            };
            effect(host, "clipboard:write", vec![json!(result)]).await?;
            effect(host, "window:hide", vec![]).await?;
            Ok(no_view(message))
        }
        ("index", _) if state["active"] != true => Err("Base64 session is not active".into()),
        ("index", "extension:search-text-changed") => {
            let input = payload["searchText"]
                .as_str()
                .ok_or("Missing search text")?;
            update_input(state, input)?;
            Ok(view(state))
        }
        ("index", "extension:refresh-session") => Ok(view(state)),
        ("index", "extension:load-more") => Ok(unchanged()),
        ("index", "extension:invoke-action") => {
            let id = payload["actionId"].as_str().ok_or("Missing action ID")?;
            let target = state["targets"]
                .get(id)
                .ok_or("Unknown or stale Base64 action")?;
            let value = target["value"]
                .as_str()
                .ok_or("Invalid Base64 action target")?
                .to_owned();
            match target["verb"].as_str() {
                Some("copyToClipboard") => {
                    effect(host, "clipboard:write", vec![json!(value)]).await?;
                    Ok(view(state))
                }
                Some("pasteInApp") => {
                    // This route writes the supplied text, hides, and sends Cmd/Ctrl+V.
                    effect(host, "core:platform:paste-text", vec![json!(value)]).await?;
                    Ok(no_view(""))
                }
                Some("openInBrowser") if is_http_url(&value) => {
                    effect(host, "shell:open", vec![json!(value)]).await?;
                    effect(host, "window:hide", vec![]).await?;
                    Ok(no_view(""))
                }
                _ => Err("Unknown Base64 action".into()),
            }
        }
        _ => Err("Unsupported Base64 operation".into()),
    }
}
