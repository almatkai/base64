//! Narrow mapping from the V2 engine's core routes to typed SDK host requests.
//!
//! Only the five effects the Base64 engine actually uses are allowed. Anything
//! else is rejected locally and never reaches the host. The manifest grants the
//! exact matching capabilities (no broad groups).
use serde_json::{json, Value};
use tezbar_base64_v2::{Host, HostFuture};
use tezbar_extension_sdk::ExtensionContext;

pub struct ProcessHost<'c>(pub &'c ExtensionContext);

fn text_arg(route: &str, args: &[Value]) -> Result<Value, String> {
    match args {
        [Value::String(text)] => Ok(json!(text)),
        _ => Err(format!("{route} expects exactly one text argument")),
    }
}

fn no_args(route: &str, args: &[Value]) -> Result<(), String> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(format!("{route} takes no arguments"))
    }
}

/// Translate an engine route + positional args into a typed host method + params.
pub fn translate(route: &str, args: &[Value]) -> Result<(&'static str, Value), String> {
    Ok(match route {
        "clipboard:read" => {
            no_args(route, args)?;
            ("host.clipboard.read", json!({}))
        }
        "clipboard:write" => (
            "host.clipboard.write",
            json!({"text": text_arg(route, args)?}),
        ),
        // Host writes the text, hides Tezbar, restores focus and sends Cmd/Ctrl+V.
        "core:platform:paste-text" => (
            "host.clipboard.pasteText",
            json!({"text": text_arg(route, args)?}),
        ),
        "shell:open" => ("host.shell.open", json!({"target": text_arg(route, args)?})),
        "window:hide" => {
            no_args(route, args)?;
            ("host.window.visibility", json!({"visible": false}))
        }
        _ => return Err(format!("Base64 host route not supported: {route}")),
    })
}

impl Host for ProcessHost<'_> {
    fn request<'a>(&'a self, route: &'a str, args: Vec<Value>) -> HostFuture<'a> {
        Box::pin(async move {
            let (method, params) = translate(route, &args)?;
            self.0.host(method, params).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::translate;
    use serde_json::json;

    #[test]
    fn maps_exactly_the_engine_routes() {
        assert_eq!(
            translate("clipboard:read", &[]).unwrap(),
            ("host.clipboard.read", json!({}))
        );
        assert_eq!(
            translate("clipboard:write", &[json!("x")]).unwrap(),
            ("host.clipboard.write", json!({"text":"x"}))
        );
        assert_eq!(
            translate("core:platform:paste-text", &[json!("é")]).unwrap(),
            ("host.clipboard.pasteText", json!({"text":"é"}))
        );
        assert_eq!(
            translate("shell:open", &[json!("https://a.b")]).unwrap(),
            ("host.shell.open", json!({"target":"https://a.b"}))
        );
        assert_eq!(
            translate("window:hide", &[]).unwrap(),
            ("host.window.visibility", json!({"visible":false}))
        );
    }

    #[test]
    fn rejects_unknown_routes_and_bad_arguments() {
        for route in [
            "clipboard:paste",
            "core:platform:selected-text",
            "core:platform:screenshot",
            "",
        ] {
            assert!(translate(route, &[]).is_err(), "{route}");
        }
        assert!(translate("clipboard:write", &[]).is_err());
        assert!(translate("clipboard:write", &[json!({"text":"x"})]).is_err());
        assert!(translate("clipboard:write", &[json!("a"), json!("b")]).is_err());
        assert!(translate("shell:open", &[json!(1)]).is_err());
        assert!(translate("core:platform:paste-text", &[json!(null)]).is_err());
        assert!(translate("clipboard:read", &[json!(1)]).is_err());
        assert!(translate("window:hide", &[json!(true)]).is_err());
    }
}
