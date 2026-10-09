//! Read-only workflow domain primitives. No approval issuer or Git executor.
pub mod input_limits;
pub mod stage;
use sha2::{Digest, Sha256};
pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
pub mod baseline;
pub mod dependencies;
pub(crate) fn valid_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|v| {
        v.len() == 64
            && v.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

pub mod approvals;

pub mod stage_transition;

pub mod obligations;
#[path = "../adapters/openspec/mod.rs"]
pub mod openspec;

pub mod action_policy;
#[path = "../adapters/specguard/mod.rs"]
pub mod specguard_adapter;

pub mod context;

pub mod projection;

pub mod gate;

pub mod policy;

pub mod run_store;

#[cfg(target_os = "linux")]
pub mod durable_run_store;

pub mod cli;

#[path = "../adapters/legacy/mod.rs"]
pub mod legacy;

pub mod evidence;
#[path = "../adapters/guards/mod.rs"]
mod guard_adapters;
pub mod release;
pub mod stage_qualification;

mod admission;
