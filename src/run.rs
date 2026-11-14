use anyhow::Context;
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub struct Run {
    #[command(flatten)]
    build: crate::build::Build,

    #[arg(last = true)]
    extra_args: Vec<String>,
}

impl Run {
    pub fn run_project(self) -> anyhow::Result<()> {
        let crate::build::BuildOutput {
            build_dir,
            profile,
            run_target,
        } = self.build.build_project()?;

        let target_name = run_target.ok_or_else(|| {
            anyhow::anyhow!("this project has no run target, set one in .cppx.json")
        })?;

        let exe_path = find_executable(&build_dir, profile.as_str(), &target_name)?;

        let status = std::process::Command::new(&exe_path)
            .args(&self.extra_args)
            .status()
            .with_context(|| format!("failed to run {exe_path:?}"))?;

        if !status.success() {
            // hand the program's own exit code back to the shell
            std::process::exit(status.code().unwrap_or(1));
        }

        Ok(())
    }
}

/// Single-config generators put the binary in the build dir itself, multi-config
/// ones (Visual Studio, Ninja Multi-Config) nest it in a per-config subdirectory.
fn find_executable(build_dir: &Path, profile: &str, target: &str) -> anyhow::Result<PathBuf> {
    let candidates = [build_dir.join(target), build_dir.join(profile).join(target)];

    candidates
        .iter()
        .map(|p| with_exe_suffix(p.clone()))
        .find(|p| p.is_file())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "expected executable at {:?}, but it wasn't found",
                with_exe_suffix(candidates[0].clone())
            )
        })
}

// a target named "my.app" would lose ".app" with set_extension, so push instead
fn with_exe_suffix(path: PathBuf) -> PathBuf {
    if cfg!(windows) {
        let mut os = path.into_os_string();
        os.push(".exe");
        PathBuf::from(os)
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_exe(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(with_exe_suffix(path.to_path_buf()), "").unwrap();
    }

    #[test]
    fn finds_single_config_executable() {
        let dir = tempfile::tempdir().unwrap();
        make_exe(&dir.path().join("app"));

        let exe = find_executable(dir.path(), "Debug", "app").unwrap();
        assert_eq!(exe, with_exe_suffix(dir.path().join("app")));
    }

    #[test]
    fn finds_multi_config_executable() {
        let dir = tempfile::tempdir().unwrap();
        make_exe(&dir.path().join("Release/app"));

        let exe = find_executable(dir.path(), "Release", "app").unwrap();
        assert_eq!(exe, with_exe_suffix(dir.path().join("Release/app")));
    }

    #[test]
    fn keeps_dots_in_target_names() {
        let dir = tempfile::tempdir().unwrap();
        make_exe(&dir.path().join("my.app"));

        assert!(find_executable(dir.path(), "Debug", "my.app").is_ok());
    }

    #[test]
    fn missing_executable_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(find_executable(dir.path(), "Debug", "app").is_err());
    }
}
