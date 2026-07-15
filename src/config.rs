use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    name: String,

    generator: Option<Generator>,

    jobs: u32,

    #[serde(rename = "build-dir")]
    build_dir: String,

    #[serde(rename = "build-type")]
    build_type: BuildType,

    #[serde(default, rename="cmake-options")]
    cmake_options: Vec<String>,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum Generator {
    Makefile,
    Ninja,
    VisualStudio,
}

#[derive(Clone, Copy, Serialize, Default, Deserialize, Debug)]
pub enum BuildType {
    #[default]
    Debug,
    Release,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            generator: None,
            jobs: 1,
            build_dir: String::from("build"),
            build_type: BuildType::default(),
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

    // write it as a json file
    #[allow(unused)]
    pub fn make_config(self, project: &std::path::Path) -> anyhow::Result<()> {
        anyhow::ensure!(project.is_dir(),
            "{project:?} doesn't exist");

        let json = serde_json::to_string_pretty(&self)?;
        let file = project.join(".cppx.json");

        std::fs::write(file, json)?;

        println!("Made config '.cppx.json'");
        Ok(())
    }

    #[allow(unused)]
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    #[allow(unused)]
    pub fn set_generator(&mut self, generator: Generator) {
        self.generator = Some(generator);
    }

    #[allow(unused)]
    pub fn set_jobs(&mut self, jobs: u32) {
        self.jobs = jobs;
    }

    #[allow(unused)]
    pub fn set_build_dir(&mut self, build_dir: String) {
        self.build_dir = build_dir;
    }

    #[allow(unused)]
    pub fn set_build_type(&mut self, build_type: BuildType) {
        self.build_type = build_type;
    }

    #[allow(unused)]
    pub fn set_cmake_options(&mut self, cmake_options: Vec<String>) {
        self.cmake_options = cmake_options;
    }
}
