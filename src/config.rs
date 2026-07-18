use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::build::{BuildType, Generator};

const CONFIG_FILE: &str = ".cppx.json";

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub name: String,

    pub generator: Option<Generator>,

    pub jobs: u16,

    #[serde(rename = "build-dir")]
    pub build_dir: std::path::PathBuf,

    #[serde(rename = "build-type")]
    pub profile: BuildType,

    #[serde(default, rename = "cmake-options")]
    pub cmake_options: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            generator: None,
            jobs: 1,
            build_dir: std::path::PathBuf::from("build"),
            profile: BuildType::default(),
            cmake_options: Vec::default(),
            name: String::default(),
        }
    }
}

impl Config {
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    pub fn get_config(project: &std::path::Path) -> anyhow::Result<Option<Self>> {
        let config = project.join(CONFIG_FILE);

        let contents = match fs::read_to_string(config) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e).context("failed to read .cppx.json"),
        };

        match serde_json::from_str(&contents) {
            Ok(config) => Ok(Some(config)),
            Err(e) => {
                Err(e).context("failed to parse .cppx.json")
            }
        }
    }

    // write it as a json file
    pub fn make_config(self, project: &std::path::Path) -> anyhow::Result<()> {
        anyhow::ensure!(project.is_dir(), "{project:?} doesn't exist");

        let json = serde_json::to_string_pretty(&self)?;
        let file = project.join(CONFIG_FILE);

        std::fs::write(file, json)?;
        Ok(())
    }
}
