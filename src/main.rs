mod build;
mod clean;
mod config;
mod new;
mod run;
mod template;

use build::Build;
use clap::{Parser, Subcommand};
use clean::Clean;
use new::New;
use run::Run;

#[derive(Parser)]
#[command(name = "cppx", version)]
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

    /// Run the project
    Run(Run),

    /// Clean build artifacts
    Clean(Clean),
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New(new) => match new.make_project() {
            Ok(()) => (),
            Err(err) => {
                eprintln!("new error: {:?}", err);
                std::process::exit(1);
            }
        },
        Commands::Build(build) => match build.build_project() {
            Ok(_) => (),
            Err(err) => {
                eprintln!("build error: {:?}", err);
                std::process::exit(1);
            }
        },
        Commands::Run(run) => match run.run_project() {
            Ok(()) => (),
            Err(err) => {
                eprintln!("run error: {:?}", err);
                std::process::exit(1);
            }
        },
        Commands::Clean(clean) => match clean.clean_project() {
            Ok(()) => (),
            Err(err) => {
                eprintln!("clean error: {:?}", err);
                std::process::exit(1);
            }
        },
    }
}
