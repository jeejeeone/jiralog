use chrono::{DateTime, FixedOffset, Local, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use inline_colorization::*;
use java_properties::read;
use java_properties::write;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::stdin;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::{stdout, BufRead, Cursor, Seek, Write};
use std::path::PathBuf;

use crate::editor::run_editor;
use crate::jira::update_time_spent;
use crate::jira::validate_jira_time_spent;
use crate::model::Configuration;
use crate::model::{self, WorklogMessage, WorklogRecord};

static WORKLOG_FILE: &str = "worklog.csv";
static COMMIT_FILE: &str = "commit_worklog";

lazy_static! {
    static ref CONFIG: Configuration = read_config().expect("Unable to load configuration");
    static ref CURRENT_MARKER: String = "current".to_string();
}

pub fn worklog_path() -> String {
    get_worklog_path()
        .to_str()
        .expect("No csv path")
        .to_string()
}

pub fn add(
    ticket: &String,
    time_spent: &String,
    description: &String,
    started_date: &DateTime<FixedOffset>,
) -> Result<WorklogRecord, Box<dyn Error>> {
    validate_jira_time_spent(time_spent)?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(get_worklog_path())
        .unwrap();

    let needs_headers = file.seek(std::io::SeekFrom::End(0))? == 0;
    let mut writer = csv::WriterBuilder::new()
        .has_headers(needs_headers)
        .from_writer(file);

    let id = model::get_nano_id();

    let item = WorklogRecord {
        ticket: ticket.clone(),
        time_spent: time_spent.clone(),
        description: description.clone(),
        started_date: started_date.clone(),
        committed: false,
        id: id.clone(),
    };

    writer.serialize(&item)?;

    Ok(item)
}

pub fn remove(id: &String) -> Result<String, Box<dyn Error>> {
    let mut worklog = read_worklog()?;
    if let Some(item_position) = worklog.iter().position(|v| &v.id == id) {
        worklog.remove(item_position);
        write_worklog(worklog)?;

        Ok(id.clone())
    } else {
        Err(format!("No worklog item {}", id).into())
    }
}

pub fn pop() -> Result<Option<WorklogRecord>, Box<dyn Error>> {
    let mut worklog = read_worklog()?;

    let item = worklog.pop();
    write_worklog(worklog)?;

    Ok(item)
}

pub fn begin(ticket: &String, description: &String) -> Result<BeginWorklog, Box<dyn Error>> {
    let current_ticket_id = current_ticket()?.map(|v| v.id);

    end_current()?;

    let added = add(
        ticket,
        &CURRENT_MARKER,
        description,
        &Local::now().fixed_offset(),
    )?;

    let previous = current_ticket_id.map(find_item).transpose()?.flatten();

    Ok(BeginWorklog {
        previous,
        current: added,
    })
}

pub fn print_current_ticket(format: &Option<String>) -> Result<WorklogMessage, Box<dyn Error>> {
    match current_ticket()? {
        Some(value) => {
            let print_format = format
                .clone()
                .unwrap_or("[%ti]: time spent=%ts".to_string());

            let msg = print_format
                .replace("%ti", &value.ticket)
                .replace("%d", &value.description)
                .replace("%ts", &get_current_duration(&value));

            Ok(WorklogMessage(msg))
        }
        None if format.is_none() => Ok(WorklogMessage("No current ticket".to_string())),
        None => empty_ok(),
    }
}

pub fn worklog_to_stdout() -> Result<WorklogMessage, Box<dyn Error>> {
    let file = File::open(get_worklog_path())?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let v = line?;
        println!("{}", v);
    }

    empty_ok()
}

fn get_current_duration(record: &WorklogRecord) -> String {
    let now = Utc::now();
    let delta = now.signed_duration_since(record.started_date);
    let delta_minutes = delta.num_minutes();

    format!("{}m", delta_minutes)
}

fn current_ticket() -> Result<Option<WorklogRecord>, Box<dyn Error>> {
    let current = read_worklog()?
        .into_iter()
        .find(|item| item.time_spent == *CURRENT_MARKER);

    Ok(current)
}

pub fn end_current() -> Result<Option<WorklogRecord>, Box<dyn Error>> {
    let mut worklog = read_worklog()?;
    let result;

    if let Some(item) = worklog
        .iter_mut()
        .find(|record| record.time_spent == *CURRENT_MARKER)
    {
        item.time_spent = get_current_duration(item);

        result = Ok(Some(item.clone()))
    } else {
        result = Ok(None)
    }

    write_worklog(worklog)?;

    result
}

fn read_worklog() -> Result<Vec<WorklogRecord>, Box<dyn Error>> {
    if let Ok(file) = File::open(worklog_path()) {
        let mut rdr = csv::Reader::from_reader(file);

        let worklog_records: Vec<WorklogRecord> = rdr.deserialize().collect::<Result<_, _>>()?;

        Ok(worklog_records)
    } else {
        Ok(Vec::new())
    }
}

fn read_worklog_uncommitted() -> Result<Vec<WorklogRecord>, Box<dyn Error>> {
    let worklog = read_worklog()?;
    Ok(worklog.into_iter().filter(|v| !v.committed).collect())
}

fn find_item(id: String) -> Result<Option<WorklogRecord>, Box<dyn Error>> {
    let worklog = read_worklog()?;
    let item = worklog.iter().find(|&v| v.id == id);

    Ok(item.cloned())
}

fn write_worklog(worklog: Vec<WorklogRecord>) -> Result<(), Box<dyn Error>> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(get_worklog_path())?;

    let mut writer = csv::WriterBuilder::new().from_writer(file);
    worklog.iter().try_for_each(|v| writer.serialize(v))?;

    writer.flush()?;

    Ok(())
}

pub fn configure() -> Result<WorklogMessage, Box<dyn Error>> {
    let read_stdin = |msg: String| {
        print!("{}", msg);
        stdout().flush().unwrap();

        let mut input = String::new();
        stdin().read_line(&mut input).expect("Failed to read line");
        input.trim_end().to_string()
    };

    let user = read_stdin("Enter Jira user: ".to_string());
    let token = read_stdin("Enter Jira token: ".to_string());
    let instance = read_stdin("Enter Jira cloud instance: ".to_string());

    let jiralog_dir = get_config_dir_path();

    if !jiralog_dir.exists() {
        fs::create_dir_all(&jiralog_dir)?;
    }

    let config_path = get_config_path();

    let mut config_map = HashMap::new();
    config_map.insert("token".to_string(), token);
    config_map.insert("jira_cloud_instance".to_string(), instance);
    config_map.insert("user".to_string(), user);

    let file = File::create(&config_path)?;
    write(BufWriter::new(file), &config_map)?;

    Ok(WorklogMessage(format!(
        "All good! Wrote {}/jiralog.properties",
        jiralog_dir.display()
    )))
}

pub fn commit() -> Result<WorklogMessage, Box<dyn Error>> {
    end_current()?;
    let worklog_uncommitted: Vec<WorklogRecord> = read_worklog_uncommitted()?;

    if !worklog_uncommitted.is_empty() {
        let commit_worklog = run_editor(
            worklog_uncommitted.iter().collect(),
            &CONFIG.get_editor_command(),
            &get_commit_path(),
        )?;

        if commit_worklog.is_empty() {
            return Ok(WorklogMessage("Abort commit".to_string()));
        }

        let pb = ProgressBar::new(worklog_uncommitted.len() as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.white} {msg:15} [{bar:80.white/gray}] ({pos}/{len})",
            )
            .unwrap(),
        );

        let update = |item: &WorklogRecord| -> Result<(), Box<dyn Error>> {
            pb.set_message(item.ticket.clone());

            update_time_spent(&CONFIG.get_jira_url(), &CONFIG.user, &CONFIG.token, item)
                .map(|_| ())?;

            let commit_item = WorklogRecord {
                committed: true,
                ..item.clone()
            };

            update_item(&commit_item)?;

            pb.inc(1);

            Ok(())
        };

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(Cursor::new(commit_worklog.join("\n")));
        let to_commit: Vec<WorklogRecord> = rdr.deserialize().collect::<Result<_, _>>()?;

        to_commit.iter().try_for_each(update)?;

        Ok(WorklogMessage("All done".to_string()))
    } else {
        Ok(WorklogMessage("Nothing to commit".to_string()))
    }
}

pub fn update_item(item: &WorklogRecord) -> Result<(), Box<dyn Error>> {
    let mut worklog = read_worklog()?;

    if let Some(index) = worklog.iter().position(|r| r.id == item.id) {
        worklog[index] = item.clone();
        write_worklog(worklog)?;
    }

    Ok(())
}

pub fn print_info() -> Result<WorklogMessage, Box<dyn Error>> {
    let items = read_worklog()?;
    let uncommitted_items = read_worklog_uncommitted()?;

    let header = "
     ____.__              .__                 
    |    |__|___________  |  |   ____   ____  
    |    |  \\_  __ \\__  \\ |  |  /  _ \\ / ___\\ 
/\\__|    |  ||  | \\// __ \\|  |_(  <_> ) /_/  >
\\________|__||__|  (____  /____/\\____/\\___  / 
                        \\/           /_____/  
    ";

    println!("{color_bright_magenta}{}", header);

    println!(
        "Jiralog home: 
    {}",
        get_config_dir_path().display()
    );
    println!();

    println!(
        "Configuration: 
    {}",
        get_config_path().display()
    );
    println!();

    println!(
        "Worklog: 
    {}",
        get_worklog_path().display()
    );

    println!();

    println!(
        "Total items {}, uncommitted items {}",
        items.len(),
        uncommitted_items.len()
    );

    println!("{color_reset}");

    empty_ok()
}

pub fn purge() -> Result<usize, Box<dyn Error>> {
    let worklog = read_worklog()?;

    let uncommitted = read_worklog_uncommitted()?;
    let uncommitted_length = uncommitted.len();

    write_worklog(uncommitted)?;

    Ok(worklog.len() - uncommitted_length)
}

fn empty_ok() -> Result<WorklogMessage, Box<dyn Error>> {
    Ok(WorklogMessage("".to_string()))
}

fn get_config_dir_path() -> PathBuf {
    let home_dir = dirs::home_dir().expect("Could not locate home directory");

    let mut jiralog_dir = PathBuf::from(&home_dir);
    jiralog_dir.push(".jiralog");

    jiralog_dir
}

fn get_config_path() -> PathBuf {
    let jiralog_dir = get_config_dir_path();

    let mut config_path = PathBuf::from(&jiralog_dir);
    config_path.push("jiralog.properties");

    config_path
}

fn get_worklog_path() -> PathBuf {
    let mut config_dir = get_config_dir_path();
    config_dir.push(WORKLOG_FILE);

    config_dir
}

fn get_commit_path() -> PathBuf {
    let mut config_dir = get_config_dir_path();
    config_dir.push(COMMIT_FILE);

    config_dir
}

fn read_config() -> Result<Configuration, Box<dyn Error>> {
    let config = File::open(get_config_path())?;
    let config_map = read(BufReader::new(config))?;

    let token = config_map.get("token").expect("No token found");
    let jira_url = config_map.get("jira_url");
    let jira_cloud_instance = config_map.get("jira_cloud_instance");
    let user = config_map.get("user").expect("User not configured");
    let editor = config_map.get("editor");

    Ok(Configuration {
        token: token.to_string(),
        jira_url: jira_url.cloned(),
        jira_cloud_instance: jira_cloud_instance.cloned(),
        user: user.to_string(),
        editor: editor.cloned(),
    })
}

pub struct BeginWorklog {
    pub previous: Option<WorklogRecord>,
    pub current: WorklogRecord,
}

