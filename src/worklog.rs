use chrono::{DateTime, FixedOffset, Local, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{stdout, BufRead, Cursor, Seek, Write};
use std::io::stdin;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use dirs;
use java_properties::write;
use std::io::BufWriter;
use java_properties::read;
use std::io::BufReader;
use inline_colorization::*;

use crate::editor::run_editor;
use crate::model::{self, WorklogMessage, WorklogRecord};
use crate::model::Configuration;
use crate::jira::validate_jira_time_spent;
use crate::jira::update_time_spent;

static WORKLOG_FILE: &str = "worklog.csv";
static COMMIT_FILE: &str = "commit_worklog";

use lazy_static::lazy_static;

lazy_static! {
    static ref CONFIG: Configuration = read_config().expect("Unable to load configuration");
}

pub fn worklog_path() -> String {
    get_worklog_path().to_str().expect("No csv path").to_string()
}

pub fn add(ticket: String, time_spent: String, description: String, started_date: DateTime<FixedOffset>)  -> Result<WorklogRecord, Box<dyn Error>>  {
    validate_jira_time_spent(&time_spent)?;

    let mut file = OpenOptions::new()
        .write(true)
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
        started_date: started_date,
        committed: false,
        id: id.clone(),
    };

    writer.serialize(item.clone())?;

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

pub fn begin(ticket: String, description: String) -> Result<BeginWorklog, Box<dyn Error>> {
    let current_ticket = current_ticket()?;
    end_current()?;
    let added = add(ticket.clone(), "current".to_string(), description.clone(), Local::now().fixed_offset())?;

    Ok(
        BeginWorklog {
            previous: current_ticket,
            current: added
        }
    )
}

pub fn print_current_ticket()  -> Result<WorklogMessage, Box<dyn Error>> {
    if let Some(value) = current_ticket()? {
        Ok(WorklogMessage(format!("{}", format!("[{}]: duration={}", value.ticket, get_current_duration(&value)))))
    } else {
        Ok(WorklogMessage("No current ticket".to_string()))    
    }
}

pub fn worklog_to_stdout() -> Result<WorklogMessage, Box<dyn Error>> {
    let file = File::open(&get_worklog_path())?;
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
        .filter(|item| item.time_spent == "current")
        .next();
        
    Ok(current)
}

pub fn end_current() -> Result<Option<WorklogRecord>, Box<dyn Error>> {
    let mut worklog = read_worklog()?;
    let mut result;

    if let Some(item) = worklog.iter_mut().find(|record| record.time_spent == "current") {
        item.time_spent = get_current_duration(&item);
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

        let worklog_records: Vec<WorklogRecord> = rdr
            .deserialize()
            .collect::<Result<_, _>>()?;
        
        Ok(worklog_records)
    } else {
        Ok(Vec::new())
    }
}

fn read_worklog_uncommitted() -> Result<Vec<WorklogRecord>, Box<dyn Error>> {
    let worklog = read_worklog()?;
    Ok(worklog.into_iter().filter(|v| !v.committed).collect())
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

        let mut input = String::new(); // Create a mutable String to store the input
        stdin()
            .read_line(&mut input) // Read a line of input into the String
            .expect("Failed to read line");
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

    let mut src_map1 = HashMap::new();
    src_map1.insert("token".to_string(), token);
    src_map1.insert("jira_cloud_instance".to_string(), instance);
    src_map1.insert("user".to_string(), user);

    let file = File::create(&config_path)?;
    write(BufWriter::new(file), &src_map1)?;

    Ok(WorklogMessage(format!("All good! Wrote {}/jiralog.properties", jiralog_dir.display())))
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
    let mut config = File::open(get_config_path())?;
    let config_map = read(BufReader::new(config))?;
    
    let token = config_map.get("token").expect("No token found");
    let jira_url = config_map.get("jira_url");
    let jira_cloud_instance = config_map.get("jira_cloud_instance");
    let user = config_map.get("user").expect("User not configured");
    let editor = config_map.get("editor");

    Ok(
        Configuration {
            token: token.to_string(), 
            jira_url: jira_url.cloned(), 
            jira_cloud_instance: jira_cloud_instance.cloned(), 
            user: user.to_string(),
            editor: editor.cloned(),
        }
    )
}

pub fn commit() -> Result<WorklogMessage, Box<dyn Error>> {
    end_current()?;
    let worklog_uncommitted: Vec<WorklogRecord> = read_worklog_uncommitted()?;

    if !worklog_uncommitted.is_empty() {
        let commit_worklog = run_editor(worklog_uncommitted.iter().collect(), &CONFIG.get_editor_command(), &get_commit_path())?;

        if commit_worklog.is_empty() {
            return Ok(WorklogMessage("Abort commit".to_string()));
        }

        let pb = ProgressBar::new(worklog_uncommitted.len() as u64);
        pb.set_style(
            ProgressStyle::with_template("{spinner:.white} {msg:15} [{bar:80.white/gray}] ({pos}/{len})").unwrap()
        );

        let update = |item: &WorklogRecord| -> Result<(), Box <dyn Error>> {
            pb.set_message(item.ticket.clone());

            update_time_spent(&CONFIG.get_jira_url(), &CONFIG.user, &CONFIG.token, item).map(|_|())?;

            let commit_item = WorklogRecord {
                committed: true,
                ..item.clone()
            };

            update_item(&commit_item)?;

            thread::sleep(Duration::from_millis(500));
            
            pb.inc(1);

            Ok(())
        };

        let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_reader(Cursor::new(commit_worklog.join("\n")));
        let to_commit: Vec<WorklogRecord> = rdr.deserialize().collect::<Result<_, _>>()?;

        to_commit
            .iter()
            .try_for_each(update)?;

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

    let header = "
     ____.__              .__                 
    |    |__|___________  |  |   ____   ____  
    |    |  \\_  __ \\__  \\ |  |  /  _ \\ / ___\\ 
/\\__|    |  ||  | \\// __ \\|  |_(  <_> ) /_/  >
\\________|__||__|  (____  /____/\\____/\\___  / 
                        \\/           /_____/  
    ";

    println!("{color_bright_magenta}{}", header);
    
    println!("Jiralog home: 
    {}", get_config_dir_path().display());
    println!("");

    println!("Configuration: 
    {}", get_config_path().display());
    println!("");
    
    println!("Worklog: 
    {}", get_worklog_path().display());

    println!("");

    println!("Total items {}, uncommitted items {}", items.len(), 0);

    println!("{color_reset}");

    empty_ok()
}

pub struct BeginWorklog {
    pub previous: Option<WorklogRecord>,
    pub current: WorklogRecord
}

fn empty_ok() -> Result<WorklogMessage, Box<dyn Error>> {
    Ok(WorklogMessage("".to_string()))
}

pub fn purge() -> Result<usize, Box<dyn Error>> {
    let worklog = read_worklog()?;
    
    let uncommitted = read_worklog_uncommitted()?;
    let uncommitted_length = uncommitted.len();

    write_worklog(uncommitted)?;

    Ok(worklog.len() - uncommitted_length)
}