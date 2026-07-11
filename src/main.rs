mod template;

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use strfmt::strfmt;

macro_rules! hashmap {
    () => {{
        std::collections::HashMap::new()
    }};
    ( $( ($a:expr, $b:expr) ),* $(,)? ) => {{
        let mut _map = std::collections::HashMap::new();
        $(_map.insert($a, $b);)*

        _map
    }};
}

#[derive(Parser)]
#[command(name = "cppx")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args)]
struct New {
    project: PathBuf,

    #[arg(long, value_enum, ignore_case = true)]
    lang: Option<Lang>,

    #[arg(long, value_enum, ignore_case = true)]
    std: Option<Std>,

    #[arg(long = "no-clangd")]
    no_clangd: bool,

    #[arg(long = "header-only")]
    header_only: bool,
}

impl New {
    fn validate(std: Std, lang: Lang) -> Result<(), &'static str> {
        match (std, lang) {
            (Std::C89 | Std::C11 | Std::C17 | Std::C23, Lang::Cpp) => {
                Err("Invalid standard for C++")
            }

            (Std::Cpp11 | Std::Cpp17 | Std::Cpp20 | Std::Cpp23 | Std::Cpp26, Lang::C) => {
                Err("Invalid standard for C")
            }

            _ => Ok(()),
        }
    }
}

#[derive(Args)]
struct Build {
    #[arg(long)]
    release: bool,
}

// #[derive(Args)]
// struct Run {
//     path: String,
//
//     #[arg(short = 'v', long = "verbose", default_value_t = 0)]
//     verbosity: u8,
// }

#[derive(Subcommand)]
enum Commands {
    /// Create a project
    New(New),

    /// Build the project
    Build(Build),

    /// Run a file
    // Run(Run),

    /// Clean build artifacts
    Clean,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum Lang {
    #[value(name = "cpp", alias = "c++")]
    Cpp,
    C,
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

impl fmt::Display for Std {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Lang::Cpp => write!(f, "CXX"),
            Lang::C => write!(f, "C"),
        }
    }
}

fn new_project(new_args: New) -> Result<(), Box<dyn std::error::Error>> {
    let New {
        project,
        lang,
        std,
        no_clangd,
        header_only,
    } = new_args;

    if project.is_dir() {
        eprintln!("Project {:?} already exists", project);
        std::process::exit(1);
    }

    let (std, lang) = match (std, lang) {
        (None, None) => (Std::Cpp23, Lang::Cpp),
        (None, Some(lang)) => (Std::latest_for(lang), lang),
        (Some(std), None) => (std, Lang::from_std(std)),
        (Some(std), Some(lang)) => {
            New::validate(std, lang)?;

            (std, lang)
        }
    };

    let project_name = project
        .file_name()
        .ok_or_else(|| "Project path has no file name".to_string())?
        .to_str()
        .ok_or_else(|| "Project name is not valid UTF-8".to_string())?;

    let vars: HashMap<String, String> = hashmap![
        ("PROJECT".to_string(), project_name.to_string()),
        ("PROJECT_GUARD".to_string(), project_name.to_uppercase()),
        ("STD".to_string(), std.to_string()),
        ("STD_NUM".to_string(), std.get_std_num().to_string()),
        ("LANG".to_string(), lang.to_string()),
        ("LANG_EXT".to_string(), lang.get_ext().to_string()),
    ];

    let include = project.join("include");
    let mut gitignore = String::from(template::GITIGNORE);

    fs::create_dir(&project)?;
    fs::create_dir(&include)?;

    if header_only {
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

        fs::write(cmake, cmake_contents)?;
    } else {
        let src = project.join("src");
        let cmake = project.join("CMakeLists.txt");
        let cmake_contents = strfmt(template::CMAKE, &vars)?;

        fs::create_dir(src)?;
        fs::write(cmake, cmake_contents)?;
    }

    if !no_clangd {
        let clangd = project.join(".clangd");

        let contents = strfmt(template::CLANGD, &vars)?;
        fs::write(clangd, contents)?;
    }

    match lang {
        Lang::Cpp => {
            let src = if header_only {
                project.join("main.cpp")
            } else {
                project.join("src/main.cpp")
            };

            let main_cpp = strfmt(template::MAIN_CPP, &vars)?;
            fs::write(src, main_cpp)?;
        }
        Lang::C => {
            let src = if header_only {
                project.join("main.c")
            } else {
                project.join("src/main.c")
            };

            let main_c = strfmt(template::MAIN_C, &vars)?;
            fs::write(src, main_c)?;
        }
    }

    fs::write(project.join(".gitignore"), gitignore)?;

    if let Err(err) = std::process::Command::new("git")
        .arg("init")
        .current_dir(&project)
        .output() {
        eprintln!("failed to initialize git: {}", err);
    }



    println!("Created: {:?} (lang={}, std={}, no-clangd={:?}, header-only={:?})", project, lang, std, no_clangd, header_only);
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New(new_args) => match new_project(new_args) {
            Ok(()) => (),
            Err(err) => {
                eprintln!("error: {}", err);
                std::process::exit(1);
            }
        },
        Commands::Build(Build { release }) => println!("build: release={}", release),
        // Commands::Run(Run { path, verbosity }) => {
        //     println!("run: path={} verbose={}", path, verbosity)
        // }
        Commands::Clean => println!("clean:"),
    }
}
