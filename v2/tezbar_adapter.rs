//! Copy this file into native_core/extensions/v2_base64.rs when integrating.
//! Add the tezbar-base64-v2 path dependency; this module is compiled into Tezbar.
use crate::native_core::common::{CoreContext, CoreResult};
use serde_json::Value;
use tezbar_base64_v2::{Host, HostFuture};

struct TezbarHost<'a>(&'a CoreContext);

impl Host for TezbarHost<'_> {
    fn request<'a>(&'a self, route: &'a str, args: Vec<Value>) -> HostFuture<'a> {
        Box::pin(self.0.request(route, args))
    }
}

pub fn commands() -> &'static [&'static str] {
    tezbar_base64_v2::commands()
}

pub const ICON: &[u8] = tezbar_base64_v2::ICON;

pub async fn handle(
    ctx: &CoreContext,
    command: &str,
    operation: &str,
    payload: &Value,
    state: &mut Value,
) -> CoreResult {
    tezbar_base64_v2::handle(&TezbarHost(ctx), command, operation, payload, state).await
}
