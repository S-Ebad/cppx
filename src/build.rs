use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{fs, num::NonZeroU16, path::PathBuf, process::Command};

#[derive(clap::Args)]
pub struct Build {
    #[arg(short, long)]
    project: Option<PathBuf>,

    #[arg(long, default_value="debug")]
    profile: BuildType,

    #[arg(long)]
    run: bool,

    #[arg(short, long, default_value_t=NonZeroU16::MIN)]
    jobs: NonZeroU16,

    #[arg(long, conflicts_with = "jobs")]
    parallel: bool,

    #[arg(short = 'G')]
    generator: Option<Generator>,

    #[arg(long)]
    clean_first: bool,

    #[arg(short = 'D')]
    cmake_options: Vec<String>,

    #[arg(long)]
    build_args: Vec<String>,

    #[arg(long)]
    config_args: Vec<String>,

    #[arg(last = true, requires = "run")]
    extra_args: Vec<String>,
}

#[derive(Serialize, Deserialize, clap::ValueEnum, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum BuildType {
    #[default]
    Debug,
    Release,
}

#[derive(Serialize, Deserialize, clap::ValueEnum, Debug, Clone, Copy)]
pub enum Generator {
    Makefile,
    Ninja,
    VisualStudio,
}

impl Generator {
    fn as_str(self) -> &'static str {
        match self {
            Generator::Makefile => "Unix Makefiles",
            Generator::Ninja => "Ninja",
            Generator::VisualStudio => "Visual Studio 17 2022",
        }
    }
}

pub fn find_project(base_path: &std::path::Path) -> Option<PathBuf> {
    let resolved_path = fs::canonicalize(base_path).ok()?;

    resolved_path
        .ancestors()
        .find(|dir| dir.join("CMakeLists.txt").is_file())
        .map(|dir| dir.to_path_buf())
}

pub fn validate_project(path: &std::path::Path) -> anyhow::Result<()> {
    anyhow::ensure!(path.is_dir(), "{path:?} does not exist");
    anyhow::ensure!(
        path.join("CMakeLists.txt").is_file(),
        "{path:?} does not contain a CMakeLists.txt"
    );

    Ok(())
}

#[cfg(unix)]
fn symlink(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn symlink(_: &std::path::Path, _: &std::path::Path) -> std::io::Result<()> {
    Ok(())
}

fn symlink_compile_commands(build_dir: &std::path::Path) -> anyhow::Result<()> {
    let src = build_dir.join("compile_commands.json");
    let dst = build_dir.join("../compile_commands.json");

    if !src.is_file() {
        return Ok(());
    }

    if dst.symlink_metadata().is_ok() {
        fs::remove_file(&dst).with_context(|| format!("failed to delete {dst:?}"))?;
    }

    symlink(&src, &dst).context("failed to create symlink")
}

impl Build {
    pub fn build_project(self) -> anyhow::Result<()> {
        let Build {
            project,
            profile,
            run,
            jobs,
            parallel,
            generator,
            clean_first,
            mut cmake_options,
            build_args,
            config_args,
            extra_args,
        }: Build = self;

        let max_jobs = std::thread::available_parallelism()
            .map(|n| n.get() as u16)
            .unwrap_or_else(|_| {
                eprintln!("warning: could not detect available parallelism, defaulting to 1");
                1
            });

        let jobs = if parallel {
            max_jobs
        } else {
            jobs.get().min(max_jobs)
        };
        let project = project.map_or_else(
            || {
                let cwd = std::env::current_dir().context("failed to get current directory")?;

                find_project(&cwd)
                    .context("could not determine project directory (missing CMakeLists.txt)")
            },
            |path| {
                validate_project(&path)?;
                Ok(path)
            },
        )?;

        let profile_str = if profile == BuildType::Release {
            "Release"
        } else {
            "Debug"
        };

        // pre-defined cmake options here
        cmake_options.push(format!("CMAKE_BUILD_TYPE={}", profile_str));

        // relative
        let build_dir_rel = PathBuf::from("build").join(profile_str);
        let build_dir = project.join(&build_dir_rel);

        // generate project if it isn't generated already
        if !build_dir.join("CMakeCache.txt").exists() {
            let status = Command::new("cmake")
                .arg("-B")
                .arg(&build_dir_rel)
                .args(generator.map(|g| ["-G", g.as_str()]).into_iter().flatten())
                .args(
                    cmake_options
                        .into_iter()
                        .flat_map(|opt| ["-D".to_string(), opt]),
                )
                .args(config_args)
                .current_dir(&project)
                .status()
                .context("failed to run cmake")?;

            anyhow::ensure!(status.success(), "cmake configure failed with {}", status);
        }

        let status = Command::new("cmake")
            .arg("--build")
            .arg(build_dir_rel)
            .arg("-j")
            .arg(jobs.to_string())
            .args(build_args)
            .args(clean_first.then_some("--clean-first"))
            .current_dir(&project)
            .status()
            .context("failed to run cmake")?;

        anyhow::ensure!(status.success(), "cmake build failed with {}", status);
        symlink_compile_commands(&build_dir)?;

        if run {
            let filename = project
                .file_name()
                .context("project path has no file name")?
                .to_str()
                .context("failed to get executable filename")?;

            let exe_path = build_dir.join(filename);

            #[cfg(windows)]
            let exe_path = exe_path.with_extension("exe");

            anyhow::ensure!(
                exe_path.is_file(),
                "expected executable at {exe_path:?}, but it wasn't found"
            );

            let status = Command::new(&exe_path)
                .args(&extra_args)
                .status()
                .with_context(|| format!("failed to run {exe_path:?}"))?;

            anyhow::ensure!(status.success(), "program exited with {status}");
        }

        Ok(())
    }
}
