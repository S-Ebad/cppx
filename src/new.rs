use crate::template;

use serde::{Deserialize, Serialize};
use std::fs;
use strfmt::strfmt;

macro_rules! hashmap {
    ( $( ($a:expr, $b:expr) ),* $(,)? ) => {{
        [$(($a, $b),)+]
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>()
    }};
}

#[derive(clap::Args)]
pub struct New {
    project: std::path::PathBuf,

    #[arg(long, value_parser = parse_name)]
    name: Option<String>,

    #[arg(long, default_value_t=Compiler::Gcc)]
    compiler: Compiler,

    #[arg(long, value_enum, ignore_case = true)]
    lang: Option<Lang>,

    #[arg(long, value_enum, ignore_case = true)]
    std: Option<Std>,

    #[arg(long = "no-clangd")]
    no_clangd: bool,

    #[arg(long = "type", default_value_t=ProjectType::Exe)]
    type_: ProjectType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum Lang {
    #[value(name = "cpp", alias = "c++")]
    Cpp,
    C,
}

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum, Serialize, Deserialize)]
pub enum Compiler {
    #[default]
    Gcc,
    Clang,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, clap::ValueEnum)]
enum ProjectType {
    #[default]
    #[value(alias = "executable")]
    Exe,
    #[value(alias = "library")]
    Lib,
    #[value(alias = "header-only")]
    Header,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum Std {
    #[value(name = "c++11", alias = "cpp11")]
    Cpp11,

    #[value(name = "c++17", alias = "cpp17")]
    Cpp17,

    #[value(name = "c++20", alias = "cpp20")]
    Cpp20,

    #[value(name = "c++23", alias = "cpp23")]
    Cpp23,

    #[value(name = "c++26", alias = "cpp26")]
    Cpp26,

    #[value(name = "c89", aliases = ["c90", "ansi"])]
    C89,

    #[value(name = "c11")]
    C11,

    #[value(name = "c17")]
    C17,

    #[value(name = "c23")]
    C23,
}

impl Std {
    fn latest_for(lang: Lang) -> Self {
        match lang {
            Lang::Cpp => Self::Cpp23,
            Lang::C => Self::C23,
        }
    }

    fn get_std_num(&self) -> &'static str {
        match self {
            Std::Cpp11 => "11",
            Std::Cpp17 => "17",
            Std::Cpp20 => "20",
            Std::Cpp23 => "23",
            Std::Cpp26 => "26",

            Std::C89 => "89",
            Std::C11 => "11",
            Std::C17 => "17",
            Std::C23 => "23",
        }
    }
}

impl Lang {
    fn from_std(std: Std) -> Self {
        match std {
            Std::Cpp11 | Std::Cpp17 | Std::Cpp20 | Std::Cpp23 | Std::Cpp26 => Self::Cpp,
            Std::C89 | Std::C11 | Std::C17 | Std::C23 => Self::C,
        }
    }

    fn get_ext(&self) -> &'static str {
        match self {
            Lang::Cpp => ".cpp",
            Lang::C => ".c",
        }
    }
}

impl std::fmt::Display for Std {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let std = match self {
            Std::Cpp11 => "c++11",
            Std::Cpp17 => "c++17",
            Std::Cpp20 => "c++20",
            Std::Cpp23 => "c++23",
            Std::Cpp26 => "c++26",

            Std::C89 => "c89",
            Std::C11 => "c11",
            Std::C17 => "c17",
            Std::C23 => "c23",
        };

        write!(f, "{}", std)
    }
}

impl std::fmt::Display for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Lang::Cpp => write!(f, "CXX"),
            Lang::C => write!(f, "C"),
        }
    }
}

impl std::fmt::Display for Compiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Compiler::Gcc => write!(f, "gcc"),
            Compiler::Clang => write!(f, "clang"),
        }
    }
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectType::Exe => write!(f, "executable"),
            ProjectType::Lib => write!(f, "library"),
            ProjectType::Header => write!(f, "header-only"),
        }
    }
}

fn parse_name(s: &str) -> Result<String, String> {
    if s.is_empty() {
        Err("name cannot be empty".to_string())
    } else {
        Ok(s.to_string())
    }
}

fn sanitize(name: &str) -> String {
    let mut result = name
        .chars()
        .map(|c| if c == '-' { '_' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>();

    if result.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        result.insert(0, '_');
    }

    result
}

impl New {
    pub fn make_project(self) -> anyhow::Result<()> {
        let New {
            project,
            name,
            compiler,
            lang,
            std,
            no_clangd,
            type_,
        } = self;

        anyhow::ensure!(!project.exists(), "project {project:?} already exists");

        let (std, lang) = match (std, lang) {
            (None, None) => (Std::Cpp23, Lang::Cpp),
            (None, Some(lang)) => (Std::latest_for(lang), lang),
            (Some(std), None) => (std, Lang::from_std(std)),
            (Some(std), Some(lang)) => {
                New::validate(std, lang)?;

                (std, lang)
            }
        };

        let project_name = match name {
            Some(name) => name,
            None => project
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("project path has no filename"))?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("project name is not a valid UTF-8"))?
                .to_string(),
        };

        let project_ident = sanitize(&project_name);

        let compiler_bin = match (compiler, lang) {
            (Compiler::Gcc, Lang::Cpp) => "g++",
            (Compiler::Gcc, Lang::C) => "gcc",
            (Compiler::Clang, Lang::Cpp) => "clang++",
            (Compiler::Clang, Lang::C) => "clang",
        };

        let vars: std::collections::HashMap<String, String> = hashmap![
            ("PROJECT".to_string(), project_name.to_string()),
            ("PROJECT_GUARD".to_string(), project_ident.to_uppercase()),
            ("PROJECT_IDENT".to_string(), project_ident),
            ("STD".to_string(), std.to_string()),
            ("STD_NUM".to_string(), std.get_std_num().to_string()),
            ("LANG".to_string(), lang.to_string()),
            ("LANG_EXT".to_string(), lang.get_ext().to_string()),
            ("COMPILER".into(), compiler_bin.to_string()),
        ];

        // everything above can fail on bad input, so only touch the disk now
        fs::create_dir_all(&project)?;

        if !no_clangd {
            let clangd = project.join(".clangd");

            let contents = strfmt(template::CLANGD, &vars)?;
            fs::write(clangd, contents)?;
        }

        let cmake_template = match type_ {
            ProjectType::Exe => template::CMAKE_EXE,
            ProjectType::Lib => template::CMAKE_LIB,
            ProjectType::Header => template::CMAKE_HEADER,
        };
        let (src_file, hdr_file) = match lang {
            Lang::Cpp => ("main.cpp", "main.hpp"),
            Lang::C => ("main.c", "main.h"),
        };

        let cmake = project.join("CMakeLists.txt");
        let cmake_contents = strfmt(cmake_template, &vars)?;
        let include = project.join("include");
        let mut gitignore = String::from(template::GITIGNORE);

        fs::write(cmake, cmake_contents)?;
        fs::create_dir(&include)?;

        match type_ {
            ProjectType::Exe | ProjectType::Lib => {
                let src = project.join("src");
                let template = match (type_, lang) {
                    (ProjectType::Exe, Lang::Cpp) => strfmt(template::MAIN_CPP, &vars)?,
                    (ProjectType::Exe, Lang::C) => strfmt(template::MAIN_C, &vars)?,

                    (ProjectType::Lib, Lang::Cpp) => strfmt(template::LIB_CPP, &vars)?,
                    (ProjectType::Lib, Lang::C) => strfmt(template::LIB_C, &vars)?,

                    _ => unreachable!(),
                };

                fs::create_dir(&src)?;
                fs::write(src.join(src_file), template)?;
            }
            ProjectType::Header => {
                let (template_hdr, template_src) = match lang {
                    Lang::Cpp => (
                        strfmt(template::MAIN_HPP, &vars)?,
                        strfmt(template::MAIN_CPP, &vars)?,
                    ),
                    Lang::C => (
                        strfmt(template::MAIN_H, &vars)?,
                        strfmt(template::MAIN_C, &vars)?,
                    ),
                };

                fs::write(include.join(hdr_file), template_hdr)?;
                fs::write(project.join(src_file), template_src)?;
                gitignore.push_str(src_file); // ignore it so its not committed
            }
        }

        fs::write(project.join(".gitignore"), gitignore)?;

        if let Err(err) = std::process::Command::new("git")
            .arg("init")
            .current_dir(&project)
            .output()
        {
            eprintln!("failed to initialize git: {}", err);
        }

        #[cfg(windows)]
        eprintln!(
            "warning: symlinking compile_commands.json is not supported on Windows; clangd may not find it automatically"
        );

        println!(
            "Created: {} (lang={}, std={}, no-clangd={:?}, type={:?}, compiler={})",
            project_name, lang, std, no_clangd, type_, compiler_bin
        );

        {
            let mut project_name = project_name;
            let run_target = match type_ {
                ProjectType::Header => {
                    project_name.push_str("_test");
                    Some(project_name)
                }
                ProjectType::Lib => None,
                ProjectType::Exe => Some(project_name),
            };

            let config = crate::config::Config::new(run_target);
            config.make_config(&project)?;
        }

        Ok(())
    }

    fn validate(std: Std, lang: Lang) -> anyhow::Result<()> {
        match (std, lang) {
            (Std::C89 | Std::C11 | Std::C17 | Std::C23, Lang::Cpp) => {
                anyhow::bail!("Invalid standard for C++");
            }

            (Std::Cpp11 | Std::Cpp17 | Std::Cpp20 | Std::Cpp23 | Std::Cpp26, Lang::C) => {
                anyhow::bail!("Invalid standard for C");
            }

            _ => Ok(()),
        }
    }
}
