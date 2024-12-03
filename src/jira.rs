use std::error::Error;
use regex::Regex;

use crate::model::WorklogRecord;

pub fn update_time_spent(
    jira_url: &str,
    user: &str,
    api_token: &str,
    worklog: &WorklogRecord,
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/rest/api/3/issue/{}/worklog", jira_url, worklog.ticket);

    let payload = serde_json::json!({
        "timeSpent": worklog.time_spent,
        "started": worklog.started_date.to_rfc3339(),
    });

    let response = client
        .post(url)
        .basic_auth(user, Some(api_token))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&payload)
        .send();

    match response {
        Ok(resp) => println!("Request succeeded with status: {}", resp.status()),
        Err(err) => println!("Request failed with error: {}", err),
    }

    Ok("".to_string())
}

pub fn validate_jira_time_spent(input: &str) -> Result<(), Box<dyn Error>> {
    if input == "current" {
        return Ok(())
    }

    let re = Regex::new(r"^(\d+[mdhw])+$").unwrap();

    if re.is_match(input) {
        Ok(())
    } else {
        Err("Invalid time spent, use jira time spent format, for example 1d5h".into())
    }
}