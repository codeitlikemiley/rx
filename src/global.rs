use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::models::config::CommandConfig;

lazy_static! {
    pub static ref CONFIGURATION_FILE_CONTENT: Arc<Mutex<String>> =
        Arc::new(Mutex::new(String::new()));
}

pub static DEFAULT_CONFIG_PATH: OnceCell<PathBuf> = OnceCell::new();
pub static DEFAULT_RUN_CONFIG: OnceCell<CommandConfig> = OnceCell::new();
pub static DEFAULT_TEST_CONFIG: OnceCell<CommandConfig> = OnceCell::new();
pub static DEFAULT_BUILD_CONFIG: OnceCell<CommandConfig> = OnceCell::new();
pub static DEFAULT_BENCH_CONFIG: OnceCell<CommandConfig> = OnceCell::new();
pub static DEFAULT_SCRIPT_CONFIG: OnceCell<CommandConfig> = OnceCell::new();
