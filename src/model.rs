use nanoid::nanoid;

use std::env;

use chrono::{DateTime, Utc};

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct WorklogRecord {
    pub ticket: String,
    pub time_spent: String,
    pub description: String,
    pub started_date: DateTime<Utc>,
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

static alphabet: [char; 16] = [
        '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f'
];
pub fn get_nano_id() -> String {
   nanoid!(10, &alphabet)
}