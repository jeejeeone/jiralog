mod worklog;
mod jira;
mod model;
mod editor;

use chrono::Local;
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

/// Command line tool to update issue worklog in Jira
#[derive(Subcommand)]
enum Commands {
    /// Add work item, by default started date is current time
    Add {
        ticket: String,
        /// Time spent in Jira format, for example 1d5h
        time_spent: String,
        /// Provide start date for work item in format 'YYYY-MM-DDTHH:MM' or 'H:M'. H:M defaults to current day
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
    Current {
        /// Output format, ticket %ti, description %d, time spent %ts. Empty if current unavailable. For example -of "[%ti]"
        #[arg(short, long)]
        format: Option<String>
    },
    /// Commit worklog to Jira
    Commit {},
    /// Remove committed entries from worklog
    Purge {},
    /// Show worklog in explorer tui, optionally to stdout
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
                "Added {}: ticket={}, time spent={}, started_date={}, description={}",
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
                worklog::pop, 
                |popped_item| 
                    popped_item.map(|v| format!(
                        "Removed {}: ticket={}, time spent={}, description={}",
                        v.id,
                        v.ticket,
                        v.time_spent,
                        v.description,
                    ))
                    .unwrap_or("Nothing to pop".to_string())
            );
        }
        Some(Commands::Commit {}) => {
            run_with_default_msg(worklog::commit);
        }
        Some(Commands::Current { format }) => {
            if format.is_some() {
                run_with_default_plain(|| worklog::print_current_ticket(format));
            } else {
                run_with_default_msg(|| worklog::print_current_ticket(format));
            }
        }
        Some(Commands::Show { stdout }) => {
            if *stdout {
                run_with_default_msg(worklog::worklog_to_stdout);
            } else {
                match run_csvlens([&worklog::worklog_path(), "--delimiter", ","]) {
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
                            "End {}: ticket={}, time spent={}, description={}\n\nBegin {}: ticket={}, description={}",
                            previous.id,
                            previous.ticket,
                            previous.time_spent,
                            previous.description,
                            begin_worklog.current.id,
                            begin_worklog.current.ticket,
                            begin_worklog.current.description,
                        ),
                    None =>
                        format!(
                            "Begin {}: ticket={}, description={}",
                            begin_worklog.current.id,
                            begin_worklog.current.ticket,
                            begin_worklog.current.description,
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
                        format!(
                            "End {}: ticket={}, time spent={}, description={}",
                            value.id,
                            value.ticket,
                            value.time_spent,
                            value.description
                        ),
                    None => 
                        "Nothing to end".to_string()
                }
            };

            run(
                worklog::end_current,
                end_ouput
            );
        }
        Some(Commands::Configure { }) => {
            run_with_default_msg(worklog::configure);
        }
        Some(Commands::Info { }) => {
            run_with_default_msg(worklog::print_info);
        }
        Some(Commands::Purge { }) => {
            run(worklog::purge, |removed_count| format!("Removed {} items", removed_count));
        }
        None => {}
    }
}

fn run<F1, F2, T>(op: F1, output_from_ok: F2)
where
    F1: FnOnce() -> Result<T, Box<dyn Error>>,
    F2: FnOnce(T) -> String,
{
    run_impl(op, output_from_ok, false)
}

fn run_impl<F1, F2, T>(op: F1, output_from_ok: F2, plain_output: bool)
where
    F1: FnOnce() -> Result<T, Box<dyn Error>>,
    F2: FnOnce(T) -> String,
{
    match op() {
        Ok(result) => {
            let output = output_from_ok(result);
            if !output.is_empty() && !plain_output {
                println!("{color_bright_green}{}{color_reset}", output);
            } else if !output.is_empty() && plain_output {
                print!("{}", output);
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
    run(op,|v| v.0)
}

fn run_with_default_plain<F1>(op: F1)
where
    F1: FnOnce() -> Result<WorklogMessage, Box<dyn Error>>,
{
    run_impl(op, |v| v.0, true)
}