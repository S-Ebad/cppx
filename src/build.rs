use anyhow::Context;
use std::{fs, path::PathBuf, process::Command};

#[derive(clap::Args)]
pub struct Build {
    #[arg(short, long)]
    project: Option<PathBuf>,

    #[arg(short, long)]
    verbose: bool,

    #[arg(long)]
    release: bool,

    #[arg(long)]
    run: bool,

    #[arg(long)]
    build_args: Vec<String>,

    #[arg(last = true, requires = "run")]
    extra_args: Vec<String>,
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
fn symlink(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(src, dst)
}

fn symlink_compile_commands(build_dir: &std::path::Path) -> anyhow::Result<()> {
    let src = build_dir.join("compile_commands.json");
    let dst = build_dir.join("../compile_commands.json");

    if !src.is_file() {
        return Ok(());
    }

    if dst.is_file() {
        fs::remove_file(&dst).with_context(|| format!("failed to delete {dst:?}"))?;
    }

    symlink(&src, &dst).context("failed to create symlink")
}

impl Build {
    pub fn build_project(self) -> anyhow::Result<()> {
        let Build {
            release,
            project,
            verbose: _,
            run,
            build_args,
            extra_args,
        } = self;

        let project = project.map_or_else(
            || {
                let cwd = std::env::current_dir().context("failed to get current directory")?;

                find_project(&cwd)
                    .context("could not determine project directory (missing CMakeLists.txt)")
            },
            |path| {
                validate_project(&path)?;
                fs::canonicalize(path).context("failed to canonicalize the project path")
            },
        )?;

        // relative
        let build_dir_rel = PathBuf::from("build").join(if release { "release" } else { "debug" });
        let build_dir = project.join(&build_dir_rel);

        // generate project if it isn't generated already
        if !build_dir.join("CMakeCache.txt").exists() {
            let status = Command::new("cmake")
                .arg("-B")
                .arg(&build_dir)
                .current_dir(&project)
                .status()
                .context("failed to run cmake")?;

            anyhow::ensure!(status.success(), "cmake configure failed with {}", status);
        }

        let status = Command::new("cmake")
            .arg("--build")
            .arg(&build_dir_rel)
            .args(&build_args)
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
