mod new;
mod template;

use clap::{Args, Parser, Subcommand};
use new::New;

#[derive(Parser)]
#[command(name = "cppx")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args)]
struct Build {
    #[arg(long)]
    release: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a project
    New(New),

    /// Build the project
    Build(Build),

    /// Clean build artifacts
    Clean,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New(new) => match new.make_project() {
            Ok(()) => (),
            Err(err) => {
                eprintln!("error: {}", err);
                std::process::exit(1);
            }
        },
        Commands::Build(Build { release }) => println!("build: release={}", release),
        Commands::Clean => println!("clean:"),
    }
}
