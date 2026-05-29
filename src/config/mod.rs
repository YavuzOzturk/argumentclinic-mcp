pub mod models;

pub use models::Config;

use anyhow::Result;

pub fn config_path() -> anyhow::Result<std::path::PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    Ok(base.join("argumentclinic").join("config.yaml"))
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(serde_yaml::from_str(&content)?)
}

pub fn save(config: &Config) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_yaml::to_string(config)?;
    std::fs::write(path, content)?;
    Ok(())
}
