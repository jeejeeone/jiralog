use chrono::{DateTime, Utc};
use reqwest::Client;
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{stdout, BufRead, Seek, Write};
use regex::Regex;
use std::io::stdin;
use std::path::PathBuf;
use dirs;
use java_properties::write;
use std::io::BufWriter;
use java_properties::read;
use std::io::BufReader;
use inline_colorization::*;
use http::StatusCode;


#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct WorklogRecord {
    ticket: String,
    time_spent: String,
    description: String,
    started_date: DateTime<Utc>,
    committed: bool
}

struct Configuration {
    token: String
}

static WORKLOG_FILE: &str = "worklog.csv";

pub fn worklog_path() -> String {
    get_csv_path().to_str().expect("No csv path").to_string()
}

pub fn add(ticket: String, time_spent: String, description: String, started_date: DateTime<Utc>)  -> Result<String, Box<dyn Error>>  {
    validate_jira_time_spent(&time_spent)?;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(get_csv_path())
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
    };

    writer.serialize(item)?;

    let success_message = format!(
        "Added [{}]: time spent='{}', description='{}'",
        ticket,
        time_spent,
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
    let ongoing_ticket = ongoing_ticket()?;
    end()?;
    add(ticket.clone(), "ongoing".to_string(), description.clone(), Utc::now())?;

    if let Some(value) = ongoing_ticket {
        Ok(format!("Begin [{}], end [{}] with duration={}", ticket, value.ticket, get_ongoing_duration(&value)))
    } else {
        Ok(format!("Begin [{}]", ticket))
    }
}

pub fn print_ongoing_ticket()  -> Result<String, Box<dyn Error>> {
    if let Some(value) = ongoing_ticket()? {
        Ok(format!("{}", format!("[{}]: duration={}", value.ticket, get_ongoing_duration(&value))))
    } else {
        Ok("No ongoing ticket".to_string())    
    }
}

pub fn worklog_to_stdout() -> Result<String, Box<dyn Error>> {
    let file = File::open(&get_csv_path())?;
    let reader = BufReader::new(file);

    for (index, line) in reader.lines().enumerate() {
        match line {
            Ok(content) => println!("{}: {}", index, content),
            Err(e) => eprintln!("Error reading line"),
        }
    }

    Ok("".to_string())
}

fn get_ongoing_duration(record: &WorklogRecord) -> String {
    let now = Utc::now();
    let delta = now.signed_duration_since(record.started_date);
    let delta_minutes = delta.num_minutes();

    format!("{}m", delta_minutes)
}

fn ongoing_ticket() -> Result<Option<WorklogRecord>, Box<dyn Error>> {
    let ongoing = read_worklog()?
        .into_iter()
        .filter(|item| item.time_spent == "ongoing")
        .next();
        
    Ok(ongoing)
}

pub fn end() -> Result<String, Box<dyn Error>> {
    let mut worklog = read_worklog()?;
    let result: Result<String, Box<dyn Error>>;
    
    if let Some(item) = worklog.iter_mut().find(|record| record.time_spent == "ongoing") {
        item.time_spent = get_ongoing_duration(&item);
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

fn write_worklog(worklog: Vec<WorklogRecord>) -> Result<(), Box<dyn Error>> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(get_csv_path())?;

    let mut writer = csv::WriterBuilder::new().from_writer(file);

    for record in worklog {
        writer.serialize(record)?;
    }

    writer.flush()?;

    Ok(())
}

pub fn validate_jira_time_spent(input: &str) -> Result<(), Box<dyn Error>> {
    if input == "ongoing" {
        return Ok(())
    }

    let re = Regex::new(r"^(\d+[mdhw])+$").unwrap();

    if re.is_match(input) {
        Ok(())
    } else {
        Err("Invalid time spent, use jira time spent format, for example 1d5h".into())
    }
}

pub fn configure() -> Result<String, Box<dyn Error>> {
    print!("Enter Jira token: ");
    stdout().flush().unwrap();

    let mut input = String::new(); // Create a mutable String to store the input
    stdin()
        .read_line(&mut input) // Read a line of input into the String
        .expect("Failed to read line");
    input = input.trim_end().to_string();
    
    let jiralog_dir = get_config_dir_path();

    if !jiralog_dir.exists() {
        fs::create_dir_all(&jiralog_dir)?;
    }

    let config_path = get_config_path();

    let mut src_map1 = HashMap::new();
    src_map1.insert("token".to_string(), input);

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

fn get_csv_path() -> PathBuf {
    let mut config_dir = get_config_dir_path();
    config_dir.push(WORKLOG_FILE);
    
    config_dir
}

fn read_config() -> Result<Configuration, Box<dyn Error>> {
    let mut config = File::open(get_config_path())?;
    let config_map = read(BufReader::new(config))?;
    
    let token = config_map.get("token").expect("No token found");

    Ok(Configuration { token: token.to_string() })
}

pub fn commit() -> Result<String, Box<dyn Error>> {
    let worklog = read_worklog()?;

    for item in worklog {
        update_time_spent("http://localhost:8080", "jarijoki1@gmail.com", "token", item)?;
    }

    Ok("a".to_string())
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
    {}", get_csv_path().display());

    println!("");

    println!("Total items {}, uncommitted items {}", items.len(), 0);

    println!("{color_reset}");

    Ok("".to_string())
}

pub fn update_time_spent(
    jira_url: &str,
    username: &str,
    api_token: &str,
    worklog: WorklogRecord,
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/rest/api/3/issue/{}/worklog", jira_url, worklog.ticket);

    let payload = serde_json::json!({
        "timeSpent": worklog.time_spent,
        "started": worklog.started_date.to_rfc3339(),
        "comment": worklog.description,
    });


    let request = client
        .post(&url)
        .bearer_auth("token".to_string())
        .json(&payload);


    let response = request.send();

    match response {
        Ok(resp) => println!("Request succeeded with status: {}", resp.status()),
        Err(err) => println!("Request failed with error: {}", err),
    }

    Ok("".to_string())
}