use nanoid::nanoid;
use chrono::{DateTime, Utc};
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
use crate::model::{self, WorklogRecord};
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

pub fn add(ticket: String, time_spent: String, description: String, started_date: DateTime<Utc>)  -> Result<String, Box<dyn Error>>  {
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

    let item = WorklogRecord {
        ticket: ticket.clone(),
        time_spent: time_spent.clone(),
        description: description.clone(),
        started_date: started_date,
        committed: false,
        id: model::get_nano_id(),
    };

    writer.serialize(item)?;

    let success_message = format!(
        "Added [{}]: time spent='{}', started_date={}, description='{}'",
        ticket,
        time_spent,
        started_date,
        description,
    );

    Ok(success_message)
}

pub fn remove(index: usize) -> Result<String, Box<dyn Error>> {
    let mut worklog = read_worklog()?;
    if index < worklog.len() {
        let item = worklog.remove(index);  
        write_worklog(worklog)?;
        let success_message = format!(
            "Removed [{}]: time spent='{}', description='{}'",
            item.ticket,
            item.time_spent,
            item.description,
        );
    
        Ok(success_message)
    } else {
        Ok("Nothing to remove".to_string())   
    }
}

pub fn pop() -> Result<String, Box<dyn Error>> {
    let mut worklog = read_worklog()?;

    let item = worklog.pop();
    write_worklog(worklog)?;

    match item {
        Some(value) =>
            Ok(format!(
                "Removed [{}]: time spent='{}', description='{}'",
                value.ticket,
                value.time_spent,
                value.description,
            )),
        None => 
            Ok("Empty, nothing to pop".to_string())
    }
    
}

pub fn begin(ticket: String, description: String) -> Result<String, Box<dyn Error>> {
    let current_ticket = current_ticket()?;
    end_current()?;
    add(ticket.clone(), "current".to_string(), description.clone(), Utc::now())?;

    if let Some(value) = current_ticket {
        Ok(format!("Begin [{}], end [{}] with duration={}", ticket, value.ticket, get_current_duration(&value)))
    } else {
        Ok(format!("Begin [{}]", ticket))
    }
}

pub fn print_current_ticket()  -> Result<String, Box<dyn Error>> {
    if let Some(value) = current_ticket()? {
        Ok(format!("{}", format!("[{}]: duration={}", value.ticket, get_current_duration(&value))))
    } else {
        Ok("No current ticket".to_string())    
    }
}

pub fn worklog_to_stdout() -> Result<String, Box<dyn Error>> {
    let file = File::open(&get_worklog_path())?;
    let reader = BufReader::new(file);

    for (index, line) in reader.lines().enumerate() {
        match line {
            Ok(content) => println!("{}: {}", index, content),
            Err(e) => eprintln!("Error reading line"),
        }
    }

    Ok("".to_string())
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

pub fn end_current() -> Result<String, Box<dyn Error>> {
    let mut worklog = read_worklog()?;
    let result: Result<String, Box<dyn Error>>;
    
    if let Some(item) = worklog.iter_mut().find(|record| record.time_spent == "current") {
        item.time_spent = get_current_duration(&item);
        result = Ok(format!("End [{}]: time spent={}", item.ticket, item.time_spent))
    } else {
        result = Ok("Nothing to end".to_string())
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

pub fn configure() -> Result<String, Box<dyn Error>> {
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

    Ok(format!("All good! Wrote {}/jiralog.properties", jiralog_dir.display()))
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

pub fn commit() -> Result<String, Box<dyn Error>> {
    end_current()?;
    let worklog_uncommitted: Vec<WorklogRecord> = read_worklog_uncommitted()?;

    if !worklog_uncommitted.is_empty() {
        let commit_worklog = run_editor(worklog_uncommitted.iter().collect(), &CONFIG.get_editor_command(), &get_commit_path())?;

        if commit_worklog.is_empty() {
            return Ok("Abort commit!".to_string());
        }

        let pb = ProgressBar::new(worklog_uncommitted.len() as u64);
        pb.set_style(
            ProgressStyle::with_template("{spinner:.white} {msg:15} [{bar:80.white/gray}] ({pos}/{len})").unwrap()
        );

        let update = |item: &WorklogRecord| -> Result<(), Box <dyn Error>> {
            pb.set_message(item.ticket.clone());

            //update_time_spent(&config.get_jira_url(), &config.user, &config.token, item).map(|_|())?;

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

        Ok("All done!".to_string())
    } else {
        Ok("Nothing to commit".to_string())
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

pub fn print_info() -> Result<String, Box<dyn Error>> {
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

    Ok("".to_string())
}