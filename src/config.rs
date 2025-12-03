use color_eyre::eyre::eyre;
use color_eyre::{Report, Result, Section};
use log::error;
use serde::Deserialize;
use thiserror::Error;

pub mod setup;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("API key is empty")]
    EmptyApiKey,

    #[error("Local keyring failed: {message}")]
    ErrorKeyRing { message: String },

    #[error("API key is invalid")]
    InvalidApiKey,

    #[error("Unable to locate API key")]
    UnknownApiKey,

    #[error("TIMET_API_KEY not set")]
    UnsetApiKey,

    #[error(transparent)]
    Fatal(Report),
}

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(skip)]
    pub version: &'static str,
    #[serde(skip)]
    pub commit: &'static str,
    #[serde(skip)]
    pub config_location: String,
    pub default_project: Option<String>,
    pub api: Api,
}

#[derive(Deserialize, Debug)]
pub struct Api {
    #[serde(skip)]
    pub key: String,
    pub endpoint: String,
}

pub static VERSION: &str = env!("CARGO_PKG_VERSION");
pub static COMMIT: &str = env!("GIT_COMMIT_HASH");

impl Config {
    pub fn new() -> Result<Self, Report> {
        let path = locate_config()?;
        let file = &std::fs::read_to_string(format!("{}/config.toml", &path))
            .suggestion("Make sure config.toml is available either at $XDG_CONFIG_HOME or at ~/.config/timet/config.toml")?;
        let mut cfg = toml::from_str::<Config>(file)?;
        cfg.config_location = path;
        cfg.api.endpoint = set_endpoint(&cfg.api.endpoint)?;
        cfg.api.key = get_api_key()?;
        cfg.version = VERSION;
        cfg.commit = COMMIT;

        Ok(cfg)
    }
}

fn set_endpoint(endpoint: &str) -> Result<String, Report> {
    match endpoint {
        e if e.starts_with("http://") => Err(eyre!("http is not a valid protocol for endpoint")
            .suggestion("When setting the API url either set https or no protocl")),
        e if e.starts_with("https://") => {
            let endpoint_https = if e.ends_with('/') {
                e.to_string()
            } else {
                format!("{}/", e)
            };
            Ok(endpoint_https)
        }
        e => Ok(format!("https://{}/", e)),
    }
}

/// Sets the config location by attempting each option
fn locate_config() -> Result<String, Report> {
    if let Ok(mut path) = std::env::var("TIMET_CONFIG_HOME") {
        path.push_str("/timet");
        Ok(path)
    } else if let Ok(mut path) = std::env::var("XDG_CONFIG_HOME") {
        path.push_str("/timet");
        Ok(path)
    } else {
        Err(eyre!("Config path is not set")
            .suggestion("Either set TIMET_CONFIG_HOME or add timet/config.toml to XDG_CONFIG_HOME"))
    }
}

/// Saves the API key to the secure store
pub fn set_api_key(api_key: &str) -> Result<(), Report> {
    let user_name = &whoami::username();
    let entry = keyring::Entry::new("timet-tui", user_name)?;
    entry.set_secret(api_key.as_bytes())?;
    Ok(())
}

/// Retrieves the API key from the secure store
pub fn get_api_key() -> Result<String, Report> {
    if let Ok(local_api_key) = std::env::var("TIMET_API_KEY") {
        return Ok(local_api_key);
    }

    let user_name = &whoami::username();
    let entry = keyring::Entry::new("timet-tui", user_name)?;
    let api_key = entry.get_secret().map_err(|e| match e {
        keyring::Error::NoEntry => Report::new(ConfigError::EmptyApiKey)
            .wrap_err(e)
            .with_warning(move || "Key is not set, if you see this error it is a bug")
            .with_section(move || format!("Attempted username {}", user_name))
            .with_suggestion(move || {
                "Create a bug report, this should have been handles by the application"
            })
            .with_suggestion(move || "For Windows this has not been tested"),
        _ => Report::new(e),
    })?;
    String::from_utf8(api_key).map_err(|e| {
        Report::new(ConfigError::InvalidApiKey)
            .with_warning(move || e.to_string())
            .with_suggestion(move || "Run 'timet-tui configure' to reconfigure API key")
    })
}

#[cfg(test)]
mod tests {

    use crate::config::{get_api_key, locate_config, set_api_key, set_endpoint, Config};

    #[test]
    fn set_endpoint_test() {
        let no_protocol = "example.org".to_string();
        assert_eq!(
            "https://example.org/",
            set_endpoint(&no_protocol).unwrap().as_str()
        );

        let protocol = "https://example.org".to_string();
        assert_eq!("https://example.org/", set_endpoint(&protocol).unwrap());

        let protocol = "https://example.org/".to_string();
        assert_eq!("https://example.org/", set_endpoint(&protocol).unwrap());

        let protocol = "http://example.org".to_string();
        assert!(set_endpoint(&protocol).is_err());
    }

    #[test]
    fn locate_config_xdg() {
        std::env::set_var("TIMET_CONFIG_HOME", "config");
        let location = locate_config().unwrap();
        std::env::remove_var("TIMET_CONFIG_HOME");
        assert_eq!(
            "config/timet", location,
            "config path should match TIMET_CONFIG_HOME is set"
        );

        std::env::set_var("XDG_CONFIG_HOME", "testdata");
        let location = locate_config().unwrap();
        std::env::remove_var("XDG_CONFIG_HOME");
        assert_eq!(
            "testdata/timet", location,
            "config path should match XDG_CONFIG_HOME"
        );
    }

    #[test]
    fn read_config_default() {
        let file = std::fs::read_to_string("testdata/config.toml");
        assert!(file.is_ok());
        let cfg = toml::from_str::<Config>(&file.unwrap());
        assert!(cfg.is_ok());
    }

    #[ignore = "testing of secure store not required (at the moment)"]
    #[test]
    fn test_keyring() {
        let result = set_api_key("test-123");
        assert!(result.ok().is_some());

        if let Ok(secret) = get_api_key() {
            assert_eq!(secret, "test-123".to_string(), "error: {}", secret);
        }
    }
}
