mod worklog;
mod jira;
mod model;
mod editor;

use chrono::{DateTime, Local, Utc};
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

/// Command line tool to record worklog items to Jira
#[derive(Subcommand)]
enum Commands {
    /// Add work item
    Add {
        ticket: String,
        /// Time spent in Jira format, for example 1d5h
        time_spent: String,
        /// Provide start date for work item in format 'YYYY-MM-DDTHH:MM' or 'HH:MM'
        #[arg(short, long)]
        started_date: Option<String>,
        /// Add description for work
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Remove work item
    Rm {
        /// Item to remove
        id: String,
    },
    /// Remove latest work item
    Pop {},
    /// Begin work item, ends previous work, records time automatically
    Begin {
        ticket: String,
        /// Add description for work
        #[arg(short, long)]
        description: Option<String>,
    },
    /// End current work
    End { },
    /// Print current work item
    Current {},
    /// Commit worklog to Jira
    Commit {},
    /// Remove committed entries from worklog
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
                    .map(|v| model::get_started_date(&v))
                    .unwrap_or_else(|| Ok(Local::now().fixed_offset()))?
            ));
        }
        Some(Commands::Rm { id }) => {
            run(|| worklog::remove(**&item_index as usize - 1));
        }
        Some(Commands::Pop {}) => {
            run(|| worklog::pop());
        }
        Some(Commands::Commit {}) => {
            run(|| worklog::commit())
        }
        Some(Commands::Current {}) => {
            run(|| worklog::print_current_ticket());
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
            run(|| worklog::end_current());
        }
        Some(Commands::Configure { }) => {
            run(|| worklog::configure())
        }
        Some(Commands::Info { }) => {
            run(|| worklog::print_info())
        }
        Some(Commands::Purge { }) => {

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
