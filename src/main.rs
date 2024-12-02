mod worklog;
use chrono::{DateTime, Utc};
use csvlens::run_csvlens;
use std::error::Error;
use inline_colorization::*;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add work item
    Add {
        ticket: String,
        /// Time spent in Jira format, for example 1d5h
        time_spent: String,
        /// Time spent in Jira format, for example 1d5h
        started_date: DateTime<Utc>,
        /// Add description for work
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Remove work item
    Rm {
        /// Item to remove
        item_index: u64,
    },
    /// Remove latest work item
    Pop {},
    /// Begin ongoing work item, records time automatically. Ends previous ongoing work.
    Begin {
        ticket: String,
        /// Add description for work
        #[arg(short, long)]
        description: Option<String>,
    },
    /// End ongoing work
    End {

    },
    /// Print current ongoing work item
    Ongoing {},
    /// Record worklog to Jira, removes successfully recorded items
    Commit {},
    /// Remove committed entries
    Purge {},
    /// Show worklog using terminal ui
    Show {
        /// Output worklog to stdout
        #[arg(short, long)]
        stdout: bool
    },
    /// Configure jiralog
    Configure {},
    /// Print info
    Info {},
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Add { ticket, time_spent, description , started_date}) => {
            run(|| worklog::add(
                ticket.to_string(), 
                time_spent.to_string(), 
                description.as_deref().unwrap_or("").to_string(),
                started_date.clone()
            ));
        }
        Some(Commands::Rm { item_index }) => {
            run(|| worklog::remove(**&item_index as usize - 1));
        }
        Some(Commands::Pop {}) => {
            run(|| worklog::pop());
        }
        Some(Commands::Commit {}) => {
            run(|| worklog::commit())
        }
        Some(Commands::Ongoing {}) => {
            run(|| worklog::print_ongoing_ticket());
        }
        Some(Commands::Show { stdout }) => {
            if *stdout {
                run(|| worklog::worklog_to_stdout());
            } else {
                match run_csvlens(&[&worklog::worklog_path(), "--delimiter", ","]) {
                    Ok(_) => {},
                    Err(e) => eprintln!("Error: {:?}", e),
                }
            }
        }
        Some(Commands::Begin { ticket, description }) => {
            run(|| worklog::begin(
                ticket.to_string(), 
                description.as_deref().unwrap_or("").to_string()
            ));
        }
        Some(Commands::End {}) => {
            run(|| worklog::end());
        }
        Some(Commands::Configure { }) => {
            run(|| worklog::configure())
        }
        Some(Commands::Info { }) => {
            run(|| worklog::print_info())
        }
        Some(Commands::Purge { }) => {
            println!("purge");
        }
        None => {}
    }
}

fn run<F>(func: F) 
where
    F: FnOnce() -> Result<String, Box<dyn Error>>
{
    match func() {
        Ok(result) => {
            if result != "" {
                println!("{color_bright_green}{}{color_reset}", result);
            }
        }
        Err(e) => {
            eprintln!("{color_bright_red}Error: {}{color_reset}", e);
        }
    }
}
