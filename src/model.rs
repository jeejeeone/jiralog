use chrono::{DateTime, Utc};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct WorklogRecord {
    pub ticket: String,
    pub time_spent: String,
    pub description: String,
    pub started_date: DateTime<Utc>,
    pub committed: bool
}

pub struct Configuration {
    pub token: String,
    pub jira_cloud_instance: Option<String>,
    pub jira_url: Option<String>,
    pub user: String
}

impl Configuration {
    pub fn get_jira_url(&self) -> String {
        self.jira_cloud_instance
            .as_ref()
            .map(|instance| format!("https://{}.atlassian.net", instance))
            .or_else(|| self.jira_url.clone())
            .expect("Configura jira_cloud_instance or jira_url")
    }
}