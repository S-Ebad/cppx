use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{fs, num::NonZeroU16, path::PathBuf, process::Command};

use crate::config::Config;

#[derive(clap::Args)]
pub struct Build {
    #[arg(short, long)]
    project: Option<PathBuf>,

    #[arg(long)]
    profile: Option<BuildType>,

    #[arg(long)]
    run: bool,

    #[arg(long)]
    reconfigure: bool,

    #[arg(short, long)]
    jobs: Option<NonZeroU16>,

    #[arg(long, conflicts_with = "jobs")]
    parallel: bool,

    #[arg(short = 'G')]
    generator: Option<Generator>,

    #[arg(long)]
    run_target: Option<String>,

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

impl BuildType {
    fn as_str(self) -> &'static str {
        match self {
            BuildType::Release => "Release",
            BuildType::Debug => "Debug",
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
    pub fn build_project(mut self) -> anyhow::Result<()> {
        let project = self.project.map_or_else(
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

        let jobs: u16;
        let build_dir_rel: std::path::PathBuf;
        let run_target: Option<String>;

        {
            let mut config = Config::get_config(&project)?.unwrap_or_default();
            let max_jobs = std::thread::available_parallelism()
                .map(|n| n.get() as u16)
                .unwrap_or_else(|_| {
                    eprintln!("warning: could not detect available parallelism, defaulting to 1");
                    1
                });

            jobs = if self.parallel {
                max_jobs
            } else {
                self.jobs.map(NonZeroU16::get).unwrap_or(config.jobs)
            };

            let profile_str = match self.profile {
                Some(x) => x.as_str(),
                None => config.profile.as_str(),
            };

            build_dir_rel = config.build_dir.join(profile_str);
            run_target = self.run_target.or(config.run_target);

            self.generator = self.generator.or(config.generator);

            config.cmake_options.extend(self.cmake_options);
            config
                .cmake_options
                .push(format!("CMAKE_BUILD_TYPE={}", profile_str));

            self.cmake_options = config.cmake_options;
        }

        // relative
        let build_dir = project.join(&build_dir_rel);

        // generate project if it isn't generated already
        if !build_dir.join("CMakeCache.txt").exists() || self.reconfigure {
            let status = Command::new("cmake")
                .arg("-B")
                .arg(&build_dir_rel)
                .args(
                    self.generator
                        .map(|g| ["-G", g.as_str()])
                        .into_iter()
                        .flatten(),
                )
                .args(
                    self.cmake_options
                        .into_iter()
                        .flat_map(|opt| ["-D".to_string(), opt]),
                )
                .args(self.config_args)
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
            .args(self.build_args)
            .args(self.clean_first.then_some("--clean-first"))
            .current_dir(&project)
            .status()
            .context("failed to run cmake")?;

        anyhow::ensure!(status.success(), "cmake build failed with {}", status);
        symlink_compile_commands(&build_dir)?;

        if self.run {
            let target_name = run_target.ok_or_else(|| anyhow::anyhow!("this project has no run target, set one in .cppx.json"))?;
            let exe_path = build_dir.join(target_name);

            #[cfg(windows)]
            let exe_path = exe_path.with_extension("exe");

            anyhow::ensure!(
                exe_path.is_file(),
                "expected executable at {exe_path:?}, but it wasn't found"
            );

            let status = Command::new(&exe_path)
                .args(&self.extra_args)
                .status()
                .with_context(|| format!("failed to run {exe_path:?}"))?;

            anyhow::ensure!(status.success(), "program exited with {status}");
        }

        Ok(())
    }
}
