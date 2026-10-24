use serde_json::{json, Value};
use std::sync::Mutex;
use tezbar_base64_v2::{decode_base64, encode_base64, handle, is_valid_base64, Host, HostFuture};

#[derive(Default)]
struct FakeHost {
    text: String,
    failure: Option<(&'static str, bool)>,
    calls: Mutex<Vec<(String, Vec<Value>)>>,
}
impl FakeHost {
    fn text(text: &str) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }
    fn fail(text: &str, route: &'static str, envelope: bool) -> Self {
        Self {
            failure: Some((route, envelope)),
            ..Self::text(text)
        }
    }
    fn calls(&self) -> Vec<(String, Vec<Value>)> {
        self.calls.lock().unwrap().clone()
    }
}
impl Host for FakeHost {
    fn request<'a>(&'a self, route: &'a str, args: Vec<Value>) -> HostFuture<'a> {
        Box::pin(async move {
            self.calls.lock().unwrap().push((route.into(), args));
            if let Some((failed, envelope)) = self.failure {
                if route == failed {
                    return if envelope {
                        Ok(json!({"ok":false,"error":"injected failure"}))
                    } else {
                        Err("injected failure".into())
                    };
                }
            }
            Ok(if route == "clipboard:read" {
                json!(self.text)
            } else {
                json!({"ok":true})
            })
        })
    }
}
fn rows(view: &Value) -> &[Value] {
    view["root"]["children"][0]["children"].as_array().unwrap()
}
fn action(view: &Value, row: usize, verb: &str) -> String {
    rows(view)[row]["props"]["actionIds"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|id| {
            let id = id.as_str().unwrap();
            id.ends_with(verb).then(|| id.to_owned())
        })
        .unwrap()
}
async fn launch(host: &FakeHost, state: &mut Value, pref: &str) -> Value {
    handle(
        host,
        "index",
        "extension:run-command",
        &json!({"preferences":{"defaultAction":pref}}),
        state,
    )
    .await
    .unwrap()
}
async fn invoke(host: &FakeHost, state: &mut Value, id: &str) -> tezbar_base64_v2::Result {
    handle(
        host,
        "index",
        "extension:invoke-action",
        &json!({"actionId":id,"value":"ATTACKER","url":"https://attacker.invalid"}),
        state,
    )
    .await
}

#[test]
fn conversions_match_original_canonical_and_padding_rules() {
    let input = "Tezbar on Windows — Сәлем 🌍 日本\0";
    assert_eq!(decode_base64(&encode_base64(input)).unwrap(), input);
    assert_eq!(encode_base64(""), "");
    for (input, expected) in [
        ("Zg", "f"),
        ("Zg=", "f"),
        ("Zg==", "f"),
        ("Zm8", "fo"),
        ("Zm8=", "fo"),
        ("==", ""),
    ] {
        assert!(is_valid_base64(input), "{input}");
        assert_eq!(decode_base64(input).unwrap(), expected);
    }
    for bad in [
        "",
        " ",
        "=",
        "===",
        "Z",
        "Zg===",
        "Zm8==",
        "Zh",
        "Zm9",
        "Zg-_",
        "Z=g=",
        "f===",
        "💥",
        "\u{0085}Zg==",
        "\u{180e}Zg==",
    ] {
        assert!(!is_valid_base64(bad), "{bad:?}");
        assert!(decode_base64(bad).is_err());
    }
    for whitespace in [
        '\t', '\n', '\u{000b}', '\u{000c}', '\r', ' ', '\u{00a0}', '\u{1680}', '\u{2000}',
        '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}',
        '\u{2008}', '\u{2009}', '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}',
        '\u{3000}', '\u{feff}',
    ] {
        assert_eq!(
            decode_base64(&format!("{whitespace}Z{whitespace}g=={whitespace}")).unwrap(),
            "f"
        );
    }
    for (bytes, expected) in [
        (&[0xff, 0xfe][..], "��"),
        (&[0xe2, 0x82][..], "�"),
        (&[0xe2, 0x28, 0xa1][..], "�(�"),
        (&[0xed, 0xa0, 0x80][..], "���"),
        (&[0xf0, 0x9f, 0x92][..], "�"),
        (&[0xc0, 0xaf][..], "��"),
    ] {
        use base64::Engine;
        assert_eq!(
            decode_base64(&base64::engine::general_purpose::STANDARD.encode(bytes)).unwrap(),
            expected
        );
    }
}

#[test]
fn deterministic_node_v1_oracle_parity() {
    let oracle: Value = serde_json::from_str(include_str!("fixtures/v1-vectors.json")).unwrap();
    for vector in oracle["vectors"].as_array().unwrap() {
        let input = vector["input"].as_str().unwrap();
        assert_eq!(encode_base64(input), vector["encoded"], "{input:?}");
        assert_eq!(is_valid_base64(input), vector["valid"], "{input:?}");
        if vector["valid"] == true {
            assert_eq!(
                decode_base64(input).unwrap(),
                vector["decoded"],
                "{input:?}"
            );
        } else {
            assert!(decode_base64(input).is_err(), "{input:?}");
        }
    }
}

#[tokio::test]
async fn live_list_preferences_previews_and_authoritative_actions() {
    let host = FakeHost::text("aGVsbG8=");
    let mut state = json!({});
    let view = launch(&host, &mut state, "pasteInApp").await;
    assert_eq!(view["ok"], true);
    assert_eq!(view["mode"], "view");
    assert_eq!(view["root"]["type"], "List");
    assert_eq!(view["root"]["props"]["__hasServerSearch"], true);
    assert_eq!(view["root"]["props"]["searchText"], "aGVsbG8=");
    assert_eq!(rows(&view).len(), 2);
    assert_eq!(rows(&view)[0]["props"]["title"], "Encode");
    assert_eq!(rows(&view)[1]["props"]["subtitle"], "hello");
    for row in rows(&view) {
        assert!(row["props"]["actionIds"][0]
            .as_str()
            .unwrap()
            .ends_with("pasteInApp"));
        for id in row["props"]["actionIds"].as_array().unwrap() {
            assert!(view["actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["id"] == *id));
        }
    }
    let id = action(&view, 1, "copyToClipboard");
    invoke(&host, &mut state, &id).await.unwrap();
    assert_eq!(
        host.calls()[1],
        ("clipboard:write".into(), vec![json!("hello")])
    );
    let refresh = handle(
        &host,
        "index",
        "extension:refresh-session",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(view, refresh); // refresh does not reread clipboard or stale existing actions
    let searched = handle(
        &host,
        "index",
        "extension:search-text-changed",
        &json!({"searchText":"one\r\n\ttwo"}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(rows(&searched).len(), 1);
    assert_eq!(
        searched["root"]["children"][0]["props"]["title"],
        "Input: one\\r\\n\\ttwo"
    );
    assert!(invoke(&host, &mut state, &id)
        .await
        .unwrap_err()
        .contains("stale"));
    assert_eq!(host.calls().len(), 2);
    let pasted = invoke(&host, &mut state, &action(&searched, 0, "pasteInApp"))
        .await
        .unwrap();
    assert_eq!(pasted["mode"], "no-view");
    assert_eq!(
        host.calls()[2],
        (
            "core:platform:paste-text".into(),
            vec![json!(encode_base64("one\r\n\ttwo"))]
        )
    );
}

#[tokio::test]
async fn browser_is_http_only_and_preference_order_is_stable() {
    for text in [
        "https://example.com/日本?q=x",
        "HTTP://example.com",
        "ftp://example.com",
        "javascript:alert(1)",
        "not a URL",
    ] {
        let host = FakeHost::text(&encode_base64(text));
        let mut state = json!({});
        let view = launch(&host, &mut state, "openInBrowser").await;
        let ids = rows(&view)[1]["props"]["actionIds"].as_array().unwrap();
        let allowed = text.starts_with("https:") || text.starts_with("HTTP:");
        assert_eq!(ids.len(), if allowed { 3 } else { 2 });
        assert!(ids[if allowed { 1 } else { 0 }]
            .as_str()
            .unwrap()
            .ends_with("copyToClipboard"));
        assert!(ids[if allowed { 2 } else { 1 }]
            .as_str()
            .unwrap()
            .ends_with("pasteInApp"));
        if allowed {
            assert!(ids[0].as_str().unwrap().ends_with("openInBrowser"));
            assert_eq!(
                invoke(&host, &mut state, ids[0].as_str().unwrap())
                    .await
                    .unwrap()["mode"],
                "no-view"
            );
            assert_eq!(host.calls()[1], ("shell:open".into(), vec![json!(text)]));
            assert_eq!(host.calls()[2].0, "window:hide");
        } else {
            assert!(invoke(&host, &mut state, "base64:1:decode:openInBrowser")
                .await
                .is_err());
            assert_eq!(host.calls().len(), 1);
        }
    }
    for pref in ["copyToClipboard", "unknown"] {
        let host = FakeHost::text("aA==");
        let mut state = json!({});
        let view = launch(&host, &mut state, pref).await;
        assert!(rows(&view)[0]["props"]["actionIds"][0]
            .as_str()
            .unwrap()
            .ends_with("copyToClipboard"));
    }
}

#[tokio::test]
async fn no_view_commands_copy_before_hide_and_validate_before_writing() {
    for (command, input, expected) in [
        ("encode", "Сәлем 🌍", encode_base64("Сәлем 🌍")),
        ("decode", "aGVsbG8", "hello".into()),
        ("decode", "==", "".into()),
        ("decode", "/w==", "�".into()),
    ] {
        let host = FakeHost::text(input);
        let result = handle(
            &host,
            command,
            "extension:run-command",
            &json!({}),
            &mut json!({}),
        )
        .await
        .unwrap();
        assert_eq!(result["mode"], "no-view");
        assert!(result.get("root").is_none());
        assert_eq!(
            host.calls(),
            vec![
                ("clipboard:read".into(), vec![]),
                ("clipboard:write".into(), vec![json!(expected)]),
                ("window:hide".into(), vec![])
            ]
        );
    }
    for (command, input) in [
        ("encode", ""),
        ("decode", ""),
        ("decode", "Zh=="),
        ("decode", "not Base64!"),
    ] {
        let host = FakeHost::text(input);
        assert!(handle(
            &host,
            command,
            "extension:run-command",
            &json!({}),
            &mut json!({})
        )
        .await
        .is_err());
        assert_eq!(host.calls(), vec![("clipboard:read".into(), vec![])]);
    }
}

#[tokio::test]
async fn host_failures_preserved_and_interactive_read_failure_has_empty_ux() {
    for command in ["encode", "decode"] {
        for (route, calls) in [
            ("clipboard:read", 1),
            ("clipboard:write", 2),
            ("window:hide", 3),
        ] {
            for envelope in [false, true] {
                let host = FakeHost::fail("aA==", route, envelope);
                assert_eq!(
                    handle(
                        &host,
                        command,
                        "extension:run-command",
                        &json!({}),
                        &mut json!({})
                    )
                    .await
                    .unwrap_err(),
                    "injected failure"
                );
                assert_eq!(host.calls().len(), calls);
            }
        }
    }
    let host = FakeHost::fail("ignored", "clipboard:read", false);
    let mut state = json!({});
    let view = launch(&host, &mut state, "copyToClipboard").await;
    assert!(view["actions"].as_array().unwrap().is_empty());
    assert_eq!(view["root"]["children"][0]["type"], "List.EmptyView");
    for (route, verb) in [
        ("clipboard:write", "copyToClipboard"),
        ("core:platform:paste-text", "pasteInApp"),
        ("shell:open", "openInBrowser"),
        ("window:hide", "openInBrowser"),
    ] {
        let host = FakeHost::fail(&encode_base64("https://example.com"), route, false);
        let mut state = json!({});
        let view = launch(&host, &mut state, "copyToClipboard").await;
        assert_eq!(
            invoke(&host, &mut state, &action(&view, 1, verb))
                .await
                .unwrap_err(),
            "injected failure"
        );
    }
}

#[tokio::test]
async fn lifecycle_search_refresh_dispose_unknown_and_stale_actions() {
    let host = FakeHost::text("aA==");
    let mut state = json!({});
    assert!(handle(
        &host,
        "bad",
        "extension:run-command",
        &json!({}),
        &mut state
    )
    .await
    .is_err());
    assert!(handle(
        &host,
        "index",
        "extension:refresh-session",
        &json!({}),
        &mut state
    )
    .await
    .is_err());
    let view = launch(&host, &mut state, "copyToClipboard").await;
    let id = action(&view, 1, "copyToClipboard");
    for invalid in [
        "clipboard:write",
        "base64:1:bad:copyToClipboard",
        "base64:1:decode:unknown",
    ] {
        assert!(invoke(&host, &mut state, invalid).await.is_err());
    }
    assert!(handle(
        &host,
        "index",
        "extension:search-text-changed",
        &json!({}),
        &mut state
    )
    .await
    .is_err());
    assert_eq!(
        handle(
            &host,
            "index",
            "extension:load-more",
            &json!({}),
            &mut state
        )
        .await
        .unwrap()["mode"],
        "unchanged"
    );
    assert_eq!(
        handle(
            &host,
            "index",
            "extension:dispose-session",
            &json!({}),
            &mut state
        )
        .await
        .unwrap()["mode"],
        "unchanged"
    );
    assert!(invoke(&host, &mut state, &id).await.is_err());
    assert!(handle(
        &host,
        "index",
        "extension:refresh-session",
        &json!({}),
        &mut state
    )
    .await
    .is_err());
    let relaunched = launch(&host, &mut state, "copyToClipboard").await;
    assert_ne!(action(&relaunched, 1, "copyToClipboard"), id);
    assert!(invoke(&host, &mut state, &id).await.is_err());
    let empty = handle(
        &host,
        "index",
        "extension:search-text-changed",
        &json!({"searchText":""}),
        &mut state,
    )
    .await
    .unwrap();
    assert!(empty["actions"].as_array().unwrap().is_empty());
    assert!(handle(
        &host,
        "encode",
        "extension:refresh-session",
        &json!({}),
        &mut state
    )
    .await
    .is_err());
    assert!(
        handle(&host, "index", "extension:invented", &json!({}), &mut state)
            .await
            .is_err()
    );
    assert_eq!(host.calls().len(), 2); // launches only; no action touched the OS
}
