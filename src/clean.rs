use std::path::PathBuf;

#[derive(clap::Args)]
pub struct Clean {
    #[arg(short, long)]
    project: Option<PathBuf>
}

impl Clean {
    pub fn clean_project(self) -> anyhow::Result<()> {
        Ok(())
    }
}
