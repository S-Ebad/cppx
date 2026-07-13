use crate::build::{find_project, validate_project};
use anyhow::Context;
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct Clean {
    #[arg(short, long)]
    project: Option<PathBuf>,
}

impl Clean {
    pub fn clean_project(self) -> anyhow::Result<()> {
        let Clean { project } = self;

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

        let build_dir = project.join("build");

        anyhow::ensure!(build_dir.is_dir(), "no build directory to clean");
        std::fs::remove_dir_all(build_dir)
            .context("failed to remove build directory")
    }
}
