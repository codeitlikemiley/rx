use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use toml;

use crate::errors::ConfigError;
use crate::global::{CONFIGURATION_FILE_CONTENT, DEFAULT_CONFIG_PATH};
use crate::helper::{read_file, write_to_config_file};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommandContext {
    Run,
    Test,
    Build,
    Bench,
    Script,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommandType {
    Cargo,
    #[default]
    Shell,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub commands: Commands,
}

impl Config {
    pub fn load(path: Option<PathBuf>) -> Result<Config, Box<dyn Error>> {
        if let Some(file_path) = path {
            read_file(file_path.as_path())?;
        } else {
            read_file(DEFAULT_CONFIG_PATH.get().unwrap())?;
        }

        let file_content = CONFIGURATION_FILE_CONTENT.lock().unwrap();

        let config: Config = toml::from_str(&file_content).unwrap_or(Config::default());
        Ok(config)
    }

    pub fn save(&self, path: Option<PathBuf>) -> Result<(), Box<dyn Error>> {
        // Determine the file path to use: provided path or default
        let file_path = path.unwrap_or_else(|| {
            DEFAULT_CONFIG_PATH
                .get()
                .expect("DEFAULT_CONFIG_PATH not set")
                .clone()
        });

        // We need Config Struct and all Other Fields (struct or enum) to be impl Serialize
        let toml_string = toml::to_string_pretty(&self)?;

        // Write the serialized string to the file line by line
        write_to_config_file(&file_path, &toml_string)?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Commands {
    pub run: Option<CommandConfig>,
    pub test: Option<CommandConfig>,
    pub build: Option<CommandConfig>,
    pub bench: Option<CommandConfig>,
    pub script: Option<CommandConfig>,
}

impl Commands {
    pub fn get_or_insert_command_config(&mut self, context: CommandContext) -> &mut CommandConfig {
        match context {
            CommandContext::Run => self.run.get_or_insert_with(CommandConfig::default),
            CommandContext::Test => self.test.get_or_insert_with(CommandConfig::default),
            CommandContext::Build => self.build.get_or_insert_with(CommandConfig::default),
            CommandContext::Bench => self.bench.get_or_insert_with(CommandConfig::default),
            CommandContext::Script => self.script.get_or_insert_with(CommandConfig::default),
        }
    }
    pub fn set_default_config(
        &mut self,
        context: CommandContext,
        new_default_key: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let command_config = match context {
            CommandContext::Run => &mut self.run,
            CommandContext::Test => &mut self.test,
            CommandContext::Build => &mut self.build,
            CommandContext::Bench => &mut self.bench,
            CommandContext::Script => &mut self.script,
        };

        if let Some(config) = command_config {
            if config.configs.contains_key(new_default_key) {
                config.default = new_default_key.to_string();
                Ok(())
            } else {
                Err(Box::new(ConfigError::ConfigKeyNotFound(
                    new_default_key.to_string(),
                )))
            }
        } else {
            Err(Box::new(ConfigError::ConfigKeyNotFound(
                new_default_key.to_string(),
            )))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CommandConfig {
    pub default: String,
    pub configs: HashMap<String, CommandDetails>,
}

impl CommandConfig {
    fn default_command_details(command: &str, command_type: CommandType) -> CommandDetails {
        CommandDetails {
            command_type,
            command: Some(command.to_string()),
            params: Some("".to_string()),
            allow_multiple_instances: Some(false),
            working_directory: Some("${workspaceFolder}".to_string()),
            pre_command: Some("".to_string()),
            env: Some(HashMap::new()),
        }
    }

    fn update_command_details<F>(&mut self, key: &str, mut update_fn: F) -> Result<(), ConfigError>
    where
        F: FnMut(&mut CommandDetails),
    {
        if let Some(details) = self.configs.get_mut(key) {
            update_fn(details);
            Ok(())
        } else {
            Err(ConfigError::ConfigKeyNotFound(key.to_string()))
        }
    }
    pub fn update_command(&mut self, key: &str, command: &str) -> Result<(), ConfigError> {
        self.update_command_details(key, |details| details.command = Some(command.to_string()))?;
        Ok(())
    }

    pub fn update_allow_multiple_instances(
        &mut self,
        key: &str,
        allow: bool,
    ) -> Result<(), ConfigError> {
        self.update_command_details(key, |details| {
            details.allow_multiple_instances = Some(allow)
        })?;
        Ok(())
    }

    pub fn update_working_directory(&mut self, key: &str, cwd: &str) -> Result<(), ConfigError> {
        self.update_command_details(key, |details| {
            details.working_directory = Some(cwd.to_string())
        })?;
        Ok(())
    }
    pub fn update_pre_command(&mut self, key: &str, pre_command: &str) -> Result<(), ConfigError> {
        // Check if trying to set pre_command to its own key
        if pre_command == key {
            return Err(ConfigError::InvalidPreCommand(format!(
                "Cannot set pre_command to its own key: {}",
                key
            )));
        }

        // Allow clearing the pre_command by setting an empty string
        if pre_command.is_empty() {
            self.update_command_details(key, |details| details.pre_command = None)?;
            return Ok(());
        }

        // Ensure the pre_command refers to an existing command key (excluding self-check above)
        if !self.configs.contains_key(pre_command) {
            return Err(ConfigError::InvalidPreCommand(format!(
                "pre_command '{}' does not exist as a command key",
                pre_command
            )));
        }

        // Proceed to update the pre_command since it passed all checks
        self.update_command_details(key, |details| {
            details.pre_command = Some(pre_command.to_string())
        })?;

        Ok(())
    }

    pub fn update_command_type(
        &mut self,
        key: &str,
        command_type: CommandType,
    ) -> Result<(), ConfigError> {
        self.update_command_details(key, |details| {
            details.command_type = command_type.clone();
        })?;

        Ok(())
    }

    pub fn update_params(&mut self, config_key: &str, new_params: &str) -> Result<(), ConfigError> {
        self.update_command_details(config_key, |details| {
            details.params = Some(new_params.to_string())
        })?;

        Ok(())
    }

    pub fn with_context(context: &str) -> Self {
        let default_details = match context {
            "run" => Self::default_command_details(
                "run --package ${packageName} --bin ${binaryName}",
                CommandType::Cargo,
            ),
            "test" => Self::default_command_details("test", CommandType::Cargo),
            "build" => Self::default_command_details("build", CommandType::Cargo),
            "bench" => Self::default_command_details("bench", CommandType::Cargo),
            _ => Self::default_command_details("", CommandType::Shell),
        };

        let mut configs = HashMap::new();
        configs.insert("default".to_string(), default_details);

        Self {
            default: "default".into(),
            configs,
        }
    }

    pub fn update_config(&mut self, key: &str, details: CommandDetails) {
        self.configs.insert(key.to_string(), details);
    }

    pub fn remove_config(&mut self, key: &str) {
        // Remove the specified config key
        self.configs.remove(key);

        // Check if the removed key was the default and reset the default if necessary
        if self.default == key {
            // Reset to a predefined fallback default key
            // Adjust this logic based on how you want to handle resetting the default
            // For example, you could check for other existing keys and choose one of them as the new default
            self.default = "default".to_string(); // Assuming "default" is a sensible fallback default key

            // Alternatively, find the first available key in `self.configs` to set as new default
            // if you prefer dynamically choosing a new default based on existing keys
            /*
            self.default = self.configs.keys().next().cloned().unwrap_or_else(|| "default".to_string());
            */
        }
    }
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            default: "default".into(),
            configs: HashMap::new(), // An empty HashMap
        }
    }
}

impl Default for Commands {
    fn default() -> Self {
        Commands {
            run: Some(CommandConfig::with_context("run")),
            test: Some(CommandConfig::with_context("test")),
            build: Some(CommandConfig::with_context("build")),
            bench: Some(CommandConfig::with_context("bench")),
            script: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct CommandDetails {
    #[serde(rename = "type")]
    pub command_type: CommandType,
    pub command: Option<String>,
    pub params: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub allow_multiple_instances: Option<bool>,
    pub working_directory: Option<String>,
    pub pre_command: Option<String>,
}
