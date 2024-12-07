mod worklog;
mod jira;
mod model;
mod editor;

use chrono::{DateTime, Local, Utc};
use csvlens::run_csvlens;
use model::{WorklogMessage, WorklogRecord};
use worklog::BeginWorklog;

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
            ), |added_item| format!(
                "Added {}: ticket='{}', time spent='{}', started_date={}, description='{}'",
                added_item.id,
                added_item.ticket,
                added_item.time_spent,
                added_item.started_date,
                added_item.description,
            ));
        }
        Some(Commands::Rm { id }) => {
            run(|| worklog::remove(id), |id| format!("Removed {}", id));
        }
        Some(Commands::Pop {}) => {
            run(
                || worklog::pop(), 
                |popped_item| 
                    popped_item.map(|v| format!(
                        "Removed {}: ticket='{}', time spent='{}', description='{}'",
                        v.id,
                        v.ticket,
                        v.time_spent,
                        v.description,
                    ))
                    .unwrap_or("Nothing to pop".to_string())
            );
        }
        Some(Commands::Commit {}) => {
            run_with_default_msg(|| worklog::commit())
        }
        Some(Commands::Current {}) => {
            run_with_default_msg(|| worklog::print_current_ticket());
        }
        Some(Commands::Show { stdout }) => {
            if *stdout {
                run_with_default_msg(|| worklog::worklog_to_stdout());
            } else {
                match run_csvlens(&[&worklog::worklog_path(), "--delimiter", ","]) {
                    Ok(_) => {},
                    Err(e) => eprintln!("Error: {:?}", e),
                }
            }
        }
        Some(Commands::Begin { ticket, description }) => {
            let begin_worklog_output = |begin_worklog: BeginWorklog| {
                match begin_worklog.previous {
                    Some(previous) => 
                        format!(
                            "End {}: time spent='{}', description='{}'\nBegin {}: ticket='{}', description='{}'", 
                            previous.id,
                            previous.time_spent, 
                            previous.description,
                            begin_worklog.current.id,
                            begin_worklog.current.ticket,
                            begin_worklog.current.description
                        ),
                    None =>
                        format!(
                            "Begin {}: ticket='{}', description='{}'", 
                            begin_worklog.current.id,
                            begin_worklog.current.ticket,
                            begin_worklog.current.description
                        )
                }
            };

            run(
                || worklog::begin(
                    ticket.to_string(), 
                    description.as_deref().unwrap_or("").to_string()
                ),
                begin_worklog_output
            );
        }
        Some(Commands::End {}) => {
            let end_ouput = |previous: Option<WorklogRecord>| {
                match previous {
                    Some(value) => 
                        format!("End {}: ticket='{}', time spent='{}'", value.id, value.ticket, value.time_spent),
                    None => 
                        "Nothing to end".to_string()
                }
            };

            run(
                || worklog::end_current(),
                end_ouput
            );
        }
        Some(Commands::Configure { }) => {
            run_with_default_msg(|| worklog::configure())
        }
        Some(Commands::Info { }) => {
            run_with_default_msg(|| worklog::print_info())
        }
        Some(Commands::Purge { }) => {

        }
        None => {}
    }
}

fn run<F1, F2, T>(op: F1, output_from_ok: F2) 
where
    F1: FnOnce() -> Result<T, Box<dyn Error>>,
    F2: FnOnce(T) -> String,
{
    match op() {
        Ok(result) => {
            let output = output_from_ok(result);
            if output != "" {
                println!("{color_bright_green}{}{color_reset}", output);
            }
        }
        Err(e) => {
            eprintln!("{color_bright_red}Error: {}{color_reset}", e);
        }
    }
}

fn run_with_default_msg<F1>(op: F1) 
where
    F1: FnOnce() -> Result<WorklogMessage, Box<dyn Error>>,
{
    run(op,|v| v.0);
}