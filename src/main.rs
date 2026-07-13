mod build;
mod clean;
mod new;
mod template;

use build::Build;
use clap::{Parser, Subcommand};
use clean::Clean;
use new::New;

#[derive(Parser)]
#[command(name = "cppx")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a project
    New(New),

    /// Build the project
    Build(Build),

    /// Clean build artifacts
    Clean(Clean),
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New(new) => match new.make_project() {
            Ok(()) => (),
            Err(err) => {
                eprintln!("new error: {}", err);
                std::process::exit(1);
            }
        },
        Commands::Build(build) => match build.build_project() {
            Ok(()) => (),
            Err(err) => {
                eprintln!("build error: {}", err);
                std::process::exit(1);
            }
        },
        Commands::Clean(clean) => match clean.clean_project() {
            Ok(()) => (),
            Err(err) => {
                eprintln!("clean error: {}", err);
                std::process::exit(1);
            }
        },
    }
}
