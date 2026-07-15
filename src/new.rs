use crate::template;

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

    #[arg(long)]
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

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
enum Compiler {
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

        anyhow::ensure!(!project.is_dir(), "project {project:?} already exists");

        let (std, lang) = match (std, lang) {
            (None, None) => (Std::Cpp23, Lang::Cpp),
            (None, Some(lang)) => (Std::latest_for(lang), lang),
            (Some(std), None) => (std, Lang::from_std(std)),
            (Some(std), Some(lang)) => {
                New::validate(std, lang)?;

                (std, lang)
            }
        };

        // FIXME: Using name currently doesn't work with `build --run`
        // Build doesn't know if you used a custom name so it still uses the project name
        let project_name = match name {
            Some(name) => name,
            None => project
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("project path has no filename"))?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("project name is not a valid UTF-8"))?
                .to_string(),
        };

        let compiler_bin = match (compiler, lang) {
            (Compiler::Gcc, Lang::Cpp) => "g++",
            (Compiler::Gcc, Lang::C) => "gcc",
            (Compiler::Clang, Lang::Cpp) => "clang++",
            (Compiler::Clang, Lang::C) => "clang",
        };

        let vars: std::collections::HashMap<String, String> = hashmap![
            ("PROJECT".to_string(), project_name.to_string()),
            ("PROJECT_GUARD".to_string(), project_name.to_uppercase()),
            ("STD".to_string(), std.to_string()),
            ("STD_NUM".to_string(), std.get_std_num().to_string()),
            ("LANG".to_string(), lang.to_string()),
            ("LANG_EXT".to_string(), lang.get_ext().to_string()),
            ("COMPILER".into(), compiler_bin.to_string()),
        ];

        let include = project.join("include");
        let mut gitignore = String::from(template::GITIGNORE);

        fs::create_dir(&project)?;
        fs::create_dir(&include)?;

        if !no_clangd {
            let clangd = project.join(".clangd");

            let contents = strfmt(template::CLANGD, &vars)?;
            fs::write(clangd, contents)?;
        }

        let src: std::path::PathBuf;
        // FIXME: Currently lib generates a src/main.cpp which contains the main method.
        // This can cause linker errors when linked with executables that already have a main
        // method.

        match type_ {
            ProjectType::Exe | ProjectType::Lib => {
                src = project.join("src");
                let cmake = project.join("CMakeLists.txt");
                let cmake_template = if type_ == ProjectType::Exe {
                    template::CMAKE
                } else {
                    template::CMAKE_LIB
                };
                let cmake_contents = strfmt(cmake_template, &vars)?;

                fs::create_dir(&src)?;
                fs::write(cmake, cmake_contents)?;
            }

            ProjectType::Header => {
                match lang {
                    Lang::Cpp => {
                        let header_contents = strfmt(template::MAIN_HPP, &vars)?;
                        fs::write(include.join("main.hpp"), header_contents)?;

                        gitignore.push_str("main.cpp");
                    }
                    Lang::C => {
                        let header_contents = strfmt(template::MAIN_H, &vars)?;
                        fs::write(include.join("main.h"), header_contents)?;

                        gitignore.push_str("main.c");
                    }
                }

                let cmake = project.join("CMakeLists.txt");
                let cmake_contents = strfmt(template::CMAKE_HEADER, &vars)?;
                src = project.clone();

                fs::write(cmake, cmake_contents)?;
            }
        }

        match lang {
            Lang::Cpp => {
                let main_cpp = src.join("main.cpp");
                fs::write(main_cpp, strfmt(template::MAIN_CPP, &vars)?)?;
            }
            Lang::C => {
                let main_c = src.join("main.c");
                fs::write(main_c, strfmt(template::MAIN_C, &vars)?)?;
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

        println!(
            "Created: {} (lang={}, std={}, no-clangd={:?}, type={:?}, compiler={})",
            project_name, lang, std, no_clangd, type_, compiler_bin
        );
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
