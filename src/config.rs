use std::path::PathBuf;

use directories::ProjectDirs;

#[derive(serde::Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub cookie: String,
    #[serde(default)]
    pub home_dir: String,
    #[serde(default)]
    pub less_usage: bool,
}

impl Config {
    pub fn Cache(&self) -> PathBuf {
        PathBuf::from(&self.home_dir).join("cache")
    }

    pub fn init(&self) {
        std::fs::create_dir_all(self.Cache()).expect("touch.cache_dir");
    }
}

pub fn load() -> Config {
    let dir = ProjectDirs::from("com", "free", "music163.lite").expect("take.project dir");
    let data_dir = dir.data_dir();
    let config_file = data_dir.join("config.yaml");
    std::fs::create_dir_all(data_dir).expect("touch.data_dir");
    let ret = Config {
        home_dir: data_dir.to_string_lossy().into_owned(),
        ..Default::default()
    };
    let config = match std::fs::read(&config_file) {
        Ok(data) => data,
        Err(e) => {
            if e.kind().eq(&std::io::ErrorKind::NotFound) {
                return ret;
            } else {
                panic!("load.config {}", e.to_string());
            }
        }
    };

    let mut newest: Config = serde_yaml::from_slice(&config).expect("parse.config");
    if newest.home_dir.is_empty() {
        newest.home_dir = ret.home_dir;
    }
    newest
}
