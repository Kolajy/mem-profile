use clap::{Parser, Subcommand};
use mem_profile::cli::{attach, cargo, run};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a command under memory profiling
    Run {
        #[arg(required = true)]
        command: String,

        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Attach to an already running process by PID
    Attach {
        #[arg(required = true)]
        pid: u32,
    },
    /// Run as a Cargo subcommand wrapper
    Cargo {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() {
    let args = Cli::parse();
    match args.command {
        Commands::Run { command, args } => {
            run::execute(command, args);
        }
        Commands::Attach { pid } => {
            attach::execute(pid);
        }
        Commands::Cargo { args } => {
            cargo::execute(args);
        }
    }
}
