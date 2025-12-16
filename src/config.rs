use serde::Deserialize;
use serde_json::from_str;
use tokio::io::AsyncReadExt;
use tokio::{fs::OpenOptions, io};

use crate::args;

#[derive(Deserialize, Default)]
pub struct Config {
  pub directories: Vec<String>,
  pub sort: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
  #[error(transparent)]
  IO(#[from] io::Error),

  #[error(transparent)]
  Json(#[from] serde_json::Error),
}

type Result<T> = std::result::Result<T, ConfigError>;

async fn get_config_from_path(config_path: &str) -> Result<Config> {
  let expanded_path = shellexpand::tilde(config_path);
  let mut file = OpenOptions::new()
    .read(true)
    .open(expanded_path.as_ref())
    .await?;
  let mut contents = String::new();
  file.read_to_string(&mut contents).await?;

  let config: Config = from_str(&contents)?;

  Ok(config)
}

pub async fn get_config() -> Config {
  let matches = args::get_matches();
  let config_path = match matches.get_one::<String>("config") {
    Some(path) => path,
    None => return Config::default(),
  };

  match get_config_from_path(config_path).await {
    Ok(config) => config,
    Err(_) => {
      eprintln!(
        "Info: No config found at {}, using default config",
        config_path
      );
      Config::default()
    }
  }
}
