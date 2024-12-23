use std::fs;
use std::path::Path;

use serde_json;
use serde_yaml;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ConfigData {
    pub target_lang: String,
    pub source_lang: String,
    pub history_lines: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct APIConfig {
    pub(crate) api_key: String,
    pub(crate) base_url: String,
}

pub fn load_api_config(config_path: &str) -> Result<APIConfig, String> {
    let yaml_str = fs::read_to_string(config_path)
        .map_err(|_| format!("无法读取配置文件: {}", config_path))?;

    let config: APIConfig = serde_yaml::from_str(&yaml_str)
        .map_err(|_| "解析配置文件失败".to_string())?;

    Ok(config)
}

pub fn load_config_from_file(input_file_path: &str) -> Option<ConfigData> {
    let config_dir = "config";
    let file_name = Path::new(input_file_path).file_name().unwrap().to_str().unwrap();
    let config_file_name = format!("{}.json", file_name.strip_suffix(".txt").unwrap());
    let config_file_path = Path::new(config_dir).join(config_file_name);

    if config_file_path.exists() {
        let file = fs::File::open(config_file_path).unwrap();
        let config_data: ConfigData = serde_json::from_reader(file).unwrap();
        Some(config_data)
    } else {
        None
    }
}