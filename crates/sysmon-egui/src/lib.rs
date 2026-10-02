#![allow(dead_code, unused_imports)]

pub mod app_shell;
pub mod monitoring;
pub mod ui;
pub use sysmon_core::app::models::*;
pub use crate::monitoring::engine::*;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
