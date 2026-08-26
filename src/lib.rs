#![allow(dead_code, unused_imports)]

pub mod app;
pub mod app_paths;
pub mod diagnostics;
pub mod monitoring;
pub mod network;
pub mod persistence;
pub mod power;
pub mod privilege;
pub mod processes;
pub mod providers;
pub mod services;
pub mod startup;
pub mod storage;
pub mod telemetry;
pub mod timeline;
pub mod ui;
pub mod updater;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use crate::app::models::*;
pub use crate::monitoring::engine::*;
