use anyhow::Context;
use std::{fs, path::PathBuf, process::Command};

#[derive(clap::Args)]
pub struct Build {
    #[arg(short, long)]
    project: Option<PathBuf>,

    #[arg(long)]
    release: bool,

    #[arg(long)]
    run: bool,

    #[arg(long)]
    build_args: Vec<String>,

    #[arg(last = true, requires = "run")]
    extra_args: Vec<String>,
}

fn find_project(base_path: PathBuf) -> Option<PathBuf> {
    let resolved_path = fs::canonicalize(base_path).ok()?;
    let mut base_path = resolved_path.as_path();

    while let Some(parent) = base_path.parent() {
        let file = base_path.join("CMakeLists.txt");

        if file.is_file() {
            return Some(parent.to_path_buf());
        }

        base_path = parent;
    }

    None
}

fn remove_file_if_exists(path: &std::path::Path) -> anyhow::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("failed to remove {path:?}")),
    }
}

fn create_compile_symlink(build_dir: &std::path::Path) -> anyhow::Result<()> {
    let src = build_dir.join("compile_commands.json");
    let dst = build_dir.join("../compile_commands.json");

    if !src.is_file() {
        return Ok(());
    }

    if dst.is_file() {
        fs::remove_file(&dst).with_context(|| format!("failed to delete {dst:?}"))?;
    }

    Ok(())
}

impl Build {
    pub fn build_project(self) -> anyhow::Result<()> {
        let Build {
            release,
            project,
            run: _,
            build_args,
            extra_args: _,
        } = self;

        let project = project
            .or_else(|| find_project(PathBuf::from(".")))
            .context("could not determine project directory (missing CMakeLists.txt)")?;

        let build_dir = project
            .join("build")
            .join(if release { "release" } else { "debug" });

        // generate project if it isn't generated already
        if !build_dir.join("CMakeCache.txt").exists() {
            let status = Command::new("cmake")
                .arg("-B")
                .arg(&build_dir)
                .status()
                .context("failed to run cmake")?;

            anyhow::ensure!(status.success(), "cmake configure failed with {}", status);
        }

        let status = Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .args(&build_args)
            .status()
            .context("failed to run cmake")?;

        anyhow::ensure!(status.success(), "cmake build failed with {}", status);

        println!("build: release={} project={:?}", release, project);
        Ok(())
    }
}
