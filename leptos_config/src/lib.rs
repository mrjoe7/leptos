pub mod errors;

use crate::errors::LeptosConfigError;
use config::{Config, File, FileFormat};
use regex::Regex;
use std::convert::TryFrom;
use std::fs;
use std::{env::VarError, net::SocketAddr, str::FromStr};
use typed_builder::TypedBuilder;

/// A Struct to allow us to parse LeptosOptions from the file. Not really needed, most interactions should
/// occur with LeptosOptions
#[derive(Clone, serde::Deserialize)]
pub struct ConfFile {
    pub leptos_options: LeptosOptions,
}
/// This struct serves as a convenient place to store details used for configuring Leptos.
/// It's used in our actix and axum integrations to generate the
/// correct path for WASM, JS, and Websockets, as well as other configuration tasks.
/// It shares keys with cargo-leptos, to allow for easy interoperability
#[derive(TypedBuilder, Clone, serde::Deserialize)]
pub struct LeptosOptions {
    /// The name of the WASM and JS files generated by wasm-bindgen. Defaults to the crate name with underscores instead of dashes
    #[builder(setter(into))]
    pub output_name: String,
    /// The path of the all the files generated by cargo-leptos
    #[builder(setter(into), default="/pkg".to_string())]
    pub site_root: String,
    /// The path of the WASM and JS files generated by wasm-bindgen from the root of your app
    /// By default, wasm-bindgen puts them in  `/pkg`.
    #[builder(setter(into), default="pkg".to_string())]
    pub site_pkg_dir: String,
    /// Used to configure the running environment of Leptos. Can be used to load dev constants and keys v prod, or change
    /// things based on the deployment environment
    /// I recommend passing in the result of `env::var("LEPTOS_ENV")`
    #[builder(setter(into), default=Env::DEV)]
    pub env: Env,
    /// Provides a way to control the address leptos is served from.
    /// Using an env variable here would allow you to run the same code in dev and prod
    /// Defaults to `127.0.0.1:3000`
    #[builder(setter(into), default=SocketAddr::from(([127,0,0,1], 3000)))]
    pub site_address: SocketAddr,
    /// The port the Websocket watcher listens on. Should match the `reload_port` in cargo-leptos(if using).
    /// Defaults to `3001`
    #[builder(default = 3001)]
    pub reload_port: u32,
}

/// An enum that can be used to define the environment Leptos is running in. Can be passed to [RenderOptions].
/// Setting this to the `PROD` variant will not include the websockets code for `cargo-leptos` watch mode.
/// Defaults to `DEV`.
#[derive(Debug, Clone, serde::Deserialize)]
pub enum Env {
    PROD,
    DEV,
}

impl Default for Env {
    fn default() -> Self {
        Self::DEV
    }
}

impl FromStr for Env {
    type Err = ();
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let sanitized = input.to_lowercase();
        match sanitized.as_ref() {
            "dev" => Ok(Self::DEV),
            "development" => Ok(Self::DEV),
            "prod" => Ok(Self::PROD),
            "production" => Ok(Self::PROD),
            _ => Ok(Self::DEV),
        }
    }
}

impl From<&str> for Env {
    fn from(str: &str) -> Self {
        let sanitized = str.to_lowercase();
        match sanitized.as_str() {
            "dev" => Self::DEV,
            "development" => Self::DEV,
            "prod" => Self::PROD,
            "production" => Self::PROD,
            _ => {
                panic!("Env var is not recognized. Maybe try `dev` or `prod`")
            }
        }
    }
}
impl From<&Result<String, VarError>> for Env {
    fn from(input: &Result<String, VarError>) -> Self {
        match input {
            Ok(str) => {
                let sanitized = str.to_lowercase();
                match sanitized.as_ref() {
                    "dev" => Self::DEV,
                    "development" => Self::DEV,
                    "prod" => Self::PROD,
                    "production" => Self::PROD,
                    _ => {
                        panic!("Env var is not recognized. Maybe try `dev` or `prod`")
                    }
                }
            }
            Err(_) => Self::DEV,
        }
    }
}

impl TryFrom<String> for Env {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "dev" => Ok(Self::DEV),
            "development" => Ok(Self::DEV),
            "prod" => Ok(Self::PROD),
            "production" => Ok(Self::PROD),
            other => Err(format!(
                "{} is not a supported environment. Use either `dev` or `production`.",
                other
            )),
        }
    }
}
/// Loads [LeptosOptions] from a Cargo.toml with layered overrides. If an env var is specified, like `LEPTOS_ENV`,
/// it will override a setting in the file.
pub async fn get_configuration(path: Option<&str>) -> Result<ConfFile, LeptosConfigError> {
    // Allow Cargo.toml path to be specified in case of workspace wonkiness
    let text = match path {
        Some(p) => fs::read_to_string(p).map_err(|_| LeptosConfigError::ConfigNotFound)?,
        None => fs::read_to_string("Cargo.toml").map_err(|_| LeptosConfigError::ConfigNotFound)?,
    };
    let re: Regex = Regex::new(r#"(?m)^\[package.metadata.leptos\]"#).unwrap();
    let start = match re.find(&text) {
        Some(found) => found.start(),
        None => return Err(LeptosConfigError::ConfigSectionNotFound),
    };

    // so that serde error messages have right line number
    let newlines = text[..start].matches('\n').count();
    let input = "\n".repeat(newlines) + &text[start..];
    let toml = input
        .replace("[package.metadata.leptos]", "[leptos_options]")
        .replace('-', "_");
    let settings = Config::builder()
        // Read the "default" configuration file
        .add_source(File::from_str(&toml, FileFormat::Toml))
        // Layer on the environment-specific values.
        // Add in settings from environment variables (with a prefix of LEPTOS and '_' as separator)
        // E.g. `LEPTOS_RELOAD_PORT=5001 would set `LeptosOptions.reload_port`
        .add_source(config::Environment::with_prefix("LEPTOS").separator("_"))
        .build()?;

    settings
        .try_deserialize()
        .map_err(|e| LeptosConfigError::ConfigError(e.to_string()))
}