use color_eyre::eyre::eyre;
use color_eyre::Result;
use serde::Deserialize;

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

impl Config {
    pub fn new() -> Result<Self> {
        let path = locate_config();
        let file = &std::fs::read_to_string(format!("{}/config.toml", &path))?;
        let mut cfg = toml::from_str::<Config>(file)?;
        cfg.config_location = path;
        cfg.api.endpoint = set_endpoint(&cfg.api.endpoint)?;
        cfg.api.key = std::env::var("TIMET_API_KEY")?;
        cfg.version = env!("CARGO_PKG_VERSION");
        cfg.commit = env!("GIT_COMMIT_HASH");

        Ok(cfg)
    }
}

fn set_endpoint(endpoint: &str) -> Result<String> {
    match endpoint {
        e if e.starts_with("http://") => Err(eyre!("http is not a valid protocol for endpoint")),
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

// Naive handling of config location
fn locate_config() -> String {
    if let Ok(mut path) = std::env::var("TIMET_CONFIG_HOME") {
        path.push_str("/timet");
        path
    } else if let Ok(mut path) = std::env::var("XDG_CONFIG_HOME") {
        path.push_str("/timet");
        path
    } else {
        panic!("NO CONFIG LOCATION FOUND");
    }
}

#[cfg(test)]
mod tests {

    use crate::config::{locate_config, set_endpoint, Config};

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
        let location = locate_config();
        std::env::remove_var("TIMET_CONFIG_HOME");
        assert_eq!(
            "config/timet", location,
            "config path should match TIMET_CONFIG_HOME is set"
        );

        std::env::set_var("XDG_CONFIG_HOME", "testdata");
        let location = locate_config();
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
}
