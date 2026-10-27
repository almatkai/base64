//! Compile the UNMODIFIED production adapter against beta's REAL common.rs.
//! Only Tauri's AppHandle/Emitter are replaced: no webview or OS actions run.
//! Reference: Tezbar origin/beta fd300e276356587726566026f072563eb9c34cb1.
extern crate self as tauri;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppHandle;
pub trait Emitter {
    fn emit(&self, channel: &str, payload: Value) -> std::result::Result<(), String>;
}
impl Emitter for AppHandle {
    fn emit(&self, _: &str, _: Value) -> std::result::Result<(), String> {
        panic!("Adapter contract tests must not use a Tauri app")
    }
}

mod native_core {
    #[allow(dead_code, clippy::all)]
    pub mod common {
        include!("fixtures/beta_common.rs");
    }
}
#[path = "../tezbar_adapter.rs"]
mod adapter;

#[test]
fn beta_manifest_schema_icon_hash_and_v1_metadata_parity() {
    let schema: Value =
        serde_json::from_str(include_str!("fixtures/extension-v2.schema.json")).unwrap();
    let manifest: Value = serde_json::from_str(tezbar_base64_v2::MANIFEST).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    validator.validate(&manifest).unwrap();
    assert_eq!(manifest["id"], "com.tezbar.native.base64");
    assert_eq!(manifest["replaces"], json!(["raycast.base64"]));
    assert_eq!(manifest["icon"], "/extensions/icons/base64.png");
    assert_eq!(adapter::commands(), &["index", "encode", "decode"]);
    assert_eq!(adapter::ICON.len(), 80527);
    assert_eq!(
        format!("{:x}", Sha256::digest(adapter::ICON)),
        "91b4ccba2a5abfaf8eb4541073a5bc10e06b8349a94b0c33538ebf8f0a2eca8c"
    );
    assert_eq!(
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("fixtures/beta_common.rs"))
        ),
        "280a0a2f0dd04a8570b320aafc0c3ece0f5de8ddb21c363645567d6a35d84da7"
    );
    assert_eq!(
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("fixtures/extension-v2.schema.json"))
        ),
        "6d5afda1b652605982104bdf16d1547547b98dfd707a7b2969316406c49f32af"
    );
    assert!(manifest["verification"]
        .as_array()
        .unwrap()
        .iter()
        .all(|v| v["state"] == "not-run"));
    assert_eq!(manifest["platforms"], json!(["macOS", "Windows"]));
    // Cargo packages remain independently testable without the surrounding V1 tree.
    let v1 = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../package.json");
    if v1.is_file() {
        let v1: Value = serde_json::from_slice(&std::fs::read(v1).unwrap()).unwrap();
        assert_eq!(manifest["preferences"], v1["preferences"]);
        for (native, original) in manifest["commands"]
            .as_array()
            .unwrap()
            .iter()
            .zip(v1["commands"].as_array().unwrap())
        {
            for key in ["name", "title", "description", "mode", "platforms"] {
                assert_eq!(native[key], original[key]);
            }
            assert_eq!(native["runtime"], "rust");
        }
        for contributor in v1["contributors"].as_array().unwrap() {
            assert!(manifest["description"]
                .as_str()
                .unwrap()
                .contains(contributor.as_str().unwrap()));
        }
    }
}

#[tokio::test]
async fn concrete_adapter_with_real_beta_corecontext_routes_all_effects() {
    use native_core::common::{CoreContext, CorePaths};
    // No files are written by CoreContext::new; paths live in the clone's target.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/adapter-fixture");
    let ctx = CoreContext::headless(CorePaths {
        data_dir: root.join("data"),
        config_dir: root.join("config"),
        resource_dir: root.join("resources"),
        temp_dir: root.join("tmp"),
        home_dir: root,
    });
    let calls = Arc::new(Mutex::new(Vec::new()));
    let log = calls.clone();
    let clipboard = Arc::new(Mutex::new(tezbar_base64_v2::encode_base64(
        "https://example.com/日本",
    )));
    let text = clipboard.clone();
    ctx.set_router(Arc::new(move |route, args| {
        let log = log.clone();
        let text = text.clone();
        Box::pin(async move {
            log.lock().unwrap().push((route.clone(), args.clone()));
            match route.as_str() {
                "clipboard:read" => {
                    assert!(args.is_empty());
                    Ok(json!(text.lock().unwrap().clone()))
                }
                "clipboard:write" | "core:platform:paste-text" | "shell:open" => {
                    assert_eq!(args.len(), 1);
                    assert!(args[0].is_string());
                    Ok(json!({"ok":true}))
                }
                "window:hide" => {
                    assert!(args.is_empty());
                    Ok(json!({"ok":true}))
                }
                _ => Err(format!("unexpected beta route {route}")),
            }
        })
    }))
    .unwrap();
    let mut state = json!({});
    let view = adapter::handle(
        &ctx,
        "index",
        "extension:run-command",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    let ids = view["root"]["children"][0]["children"][1]["props"]["actionIds"]
        .as_array()
        .unwrap();
    for verb in ["copyToClipboard", "pasteInApp", "openInBrowser"] {
        let id = ids
            .iter()
            .find(|v| v.as_str().unwrap().ends_with(verb))
            .unwrap();
        adapter::handle(
            &ctx,
            "index",
            "extension:invoke-action",
            &json!({"actionId":id}),
            &mut state,
        )
        .await
        .unwrap();
    }
    assert_eq!(
        calls.lock().unwrap().as_slice(),
        &[
            ("clipboard:read".into(), vec![]),
            (
                "clipboard:write".into(),
                vec![json!("https://example.com/日本")]
            ),
            (
                "core:platform:paste-text".into(),
                vec![json!("https://example.com/日本")]
            ),
            ("shell:open".into(), vec![json!("https://example.com/日本")]),
            ("window:hide".into(), vec![]),
        ]
    );
    for command in ["encode", "decode"] {
        calls.lock().unwrap().clear();
        *clipboard.lock().unwrap() = if command == "decode" {
            "aA==".into()
        } else {
            "h".into()
        };
        let result = adapter::handle(
            &ctx,
            command,
            "extension:run-command",
            &json!({}),
            &mut json!({}),
        )
        .await
        .unwrap();
        assert_eq!(result["mode"], "no-view");
        assert_eq!(
            calls.lock().unwrap()[1],
            (
                "clipboard:write".into(),
                vec![json!(if command == "decode" { "h" } else { "aA==" })]
            )
        );
    }
}
