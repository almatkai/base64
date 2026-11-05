//! Standalone Tezbar process executable for the Base64 extension.
//!
//! The domain engine is the reviewed Native V2 library in `v2/` (unchanged).
//! This file only adapts its `Host` route trait onto the narrow, typed
//! `host.*` reverse RPCs exposed by `tezbar-extension-sdk` and serves the
//! JSONL process protocol over stdio.
mod host;

use serde_json::Value;
use tezbar_extension_sdk::{BoxFuture, CoreResult, Extension, ExtensionContext};

pub const ID: &str = "com.tezbar.native.base64";

struct Adapter;

impl Extension for Adapter {
    fn id(&self) -> &'static str {
        ID
    }
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
    fn commands(&self) -> &'static [&'static str] {
        tezbar_base64_v2::commands()
    }
    fn handle<'a>(
        &'a self,
        ctx: &'a ExtensionContext,
        command: &'a str,
        operation: &'a str,
        payload: &'a Value,
        state: &'a mut Value,
    ) -> BoxFuture<'a, CoreResult> {
        Box::pin(async move {
            tezbar_base64_v2::handle(&host::ProcessHost(ctx), command, operation, payload, state)
                .await
        })
    }
}

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("extension runtime");
    let result = runtime.block_on(tezbar_extension_sdk::serve(Adapter));
    runtime.shutdown_timeout(std::time::Duration::from_millis(500));
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
