use crate::{Result, invalid};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
pub const DEFAULT_FONT: &str = "Maple Mono Normal NF";
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct AppConfig {
    pub font: String,
    pub restart: Option<bool>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct CustomApp {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub driver: String,
    pub process_name: String,
    pub asar_path: PathBuf,
    pub preload_path: String,
    pub unpack_flags: Vec<String>,
}
impl Default for CustomApp {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            path: PathBuf::new(),
            driver: "electron".into(),
            process_name: String::new(),
            asar_path: PathBuf::new(),
            preload_path: String::new(),
            unpack_flags: Vec::new(),
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub font: String,
    pub apps: Vec<String>,
    pub restart: bool,
    pub apps_config: BTreeMap<String, AppConfig>,
    pub custom_apps: Vec<CustomApp>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            font: DEFAULT_FONT.into(),
            apps: vec!["*".into()],
            restart: true,
            apps_config: BTreeMap::new(),
            custom_apps: Vec::new(),
        }
    }
}
impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let mut cfg: Self = toml::from_str(&fs::read_to_string(path)?)
            .map_err(|e| invalid(format!("parsing config {}: {e}", path.display())))?;
        if cfg.font.is_empty() {
            cfg.font = DEFAULT_FONT.into();
        }
        if cfg.apps.is_empty() {
            cfg.apps.push("*".into());
        }
        Ok(cfg)
    }
    pub fn ensure(path: &Path) -> Result<Self> {
        match fs::metadata(path) {
            Ok(_) => return Self::load(path),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        fs::create_dir_all(crate::parent(path))?;
        let mut file = tempfile::NamedTempFile::new_in(crate::parent(path))?;
        file.write_all(b"# better-fonts configuration\n# Per-app overrides: [apps_config.slack], font='JetBrains Mono', restart=false\nfont = 'Maple Mono Normal NF'\napps = ['*']\nrestart = true\n")?;
        file.as_file().sync_all()?;
        match file.persist_noclobber(path) {
            Ok(_) => {}
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.error.into()),
        }
        Self::load(path)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(crate::parent(path))?;
        let mut file = tempfile::NamedTempFile::new_in(crate::parent(path))?;
        file.write_all(
            toml::to_string_pretty(self)
                .map_err(|e| invalid(e.to_string()))?
                .as_bytes(),
        )?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|e| e.error)?;
        Ok(())
    }
    pub fn matches(&self, id: &str) -> bool {
        self.apps.iter().any(|s| {
            ["*", "all"].contains(&s.trim().to_ascii_lowercase().as_str())
                || s.trim().eq_ignore_ascii_case(id.trim())
        })
    }
    pub fn font_for<'a>(&'a self, id: &str, flag: Option<&'a str>) -> &'a str {
        flag.or_else(|| {
            self.apps_config
                .get(id)
                .map(|c| c.font.as_str())
                .filter(|s| !s.trim().is_empty())
        })
        .unwrap_or(if self.font.trim().is_empty() {
            DEFAULT_FONT
        } else {
            &self.font
        })
    }
    pub fn restart_for(&self, id: &str, flag: Option<bool>) -> bool {
        flag.or_else(|| self.apps_config.get(id).and_then(|c| c.restart))
            .unwrap_or(self.restart)
    }
}
pub fn config_path() -> Result<PathBuf> {
    Ok(base("XDG_CONFIG_HOME", || {
        dirs::home_dir().map(|p| p.join(".config"))
    })?
    .join("better-fonts/config.toml"))
}
pub fn cache_dir() -> Result<PathBuf> {
    Ok(base("XDG_CACHE_HOME", || {
        dirs::home_dir().map(|p| p.join(".cache"))
    })?
    .join("better-fonts"))
}
fn base(name: &str, fallback: fn() -> Option<PathBuf>) -> Result<PathBuf> {
    std::env::var_os(name)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(fallback)
        .ok_or_else(|| invalid(format!("cannot resolve {name} directory")))
}
