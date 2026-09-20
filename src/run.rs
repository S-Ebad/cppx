use anyhow::Context;

#[derive(clap::Args)]
pub struct Run {
    #[command(flatten)]
    build: crate::build::Build,

    #[arg(last = true)]
    extra_args: Vec<String>,
}

impl Run {
    pub fn run_project(self) -> anyhow::Result<()> {
        let (build_dir, run_target) = self.build.build_project()?;

        let target_name = run_target.ok_or_else(|| {
            anyhow::anyhow!("this project has no run target, set one in .cppx.json")
        })?;

        let exe_path = build_dir.join(target_name);

        // a target named "my.app" would lose ".app"
        #[cfg(windows)]
        let exe_path = {
            let mut os = exe_path.into_os_string();
            os.push(".exe");

            std::path::PathBuf::from(os)
        };

        anyhow::ensure!(
            exe_path.is_file(),
            "expected executable at {exe_path:?}, but it wasn't found"
        );

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
