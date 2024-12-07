use nanoid::nanoid;

use std::env;

use chrono::{DateTime, FixedOffset, Local, NaiveDateTime, NaiveTime, ParseError, TimeZone};

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct WorklogRecord {
    pub ticket: String,
    pub time_spent: String,
    pub description: String,
    pub started_date: DateTime<FixedOffset>,
    pub committed: bool,
    pub id: String
}

pub struct Configuration {
    pub token: String,
    pub jira_cloud_instance: Option<String>,
    pub jira_url: Option<String>,
    pub user: String,
    pub editor: Option<String>
}

pub struct WorklogMessage(pub String);

impl Configuration {
    pub fn get_jira_url(&self) -> String {
        self.jira_cloud_instance
            .as_ref()
            .map(|instance| format!("https://{}.atlassian.net", instance))
            .or_else(|| self.jira_url.clone())
            .expect("Configura jira_cloud_instance or jira_url")
    }

    pub fn get_editor_command(&self) -> String {
        env::var("EDITOR")
            .unwrap_or(self.editor.clone().unwrap_or("nano".to_string()))
    }
}

static NANO_ID_ALPHABET: [char; 16] = [
        '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f'
];
pub fn get_nano_id() -> String {
   nanoid!(10, &NANO_ID_ALPHABET)
}

pub fn get_started_date(started_date: &str) -> Result<DateTime<FixedOffset>, String> {
    let date_time = 
        NaiveDateTime::parse_from_str(started_date, "%Y-%m-%dT%H:%M")
            .or_else(|_| date_time_from_time(started_date));

    match date_time {
        Ok(value) => {
            Local::now()
                .offset()
                .from_local_datetime(&value)
                .single()
                .ok_or_else(|| "Ambiguous or invalid datetime".to_string())
        }
        Err(_) => Err("Invalid format. Use 'YYYY-MM-DDTHH:MM' or 'HH:MM'.".to_string())
    }
}

fn date_time_from_time(started_date: &str) -> Result<NaiveDateTime, ParseError> {
    let naive_time = NaiveTime::parse_from_str(started_date, "%H:%M")?;
    let today = Local::now().naive_local().date();
    
    Ok(NaiveDateTime::new(today, naive_time))
}