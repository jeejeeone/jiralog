use std::error::Error;
use regex::Regex;

use crate::model::WorklogRecord;

pub fn update_time_spent(
    jira_url: &str,
    user: &str,
    api_token: &str,
    worklog: &WorklogRecord,
) -> Result<(), Box<dyn Error>> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/rest/api/3/issue/{}/worklog", jira_url, worklog.ticket);

    let payload = serde_json::json!({
        "timeSpent": worklog.time_spent,
        "started": worklog.started_date.format("%Y-%m-%dT%H:%M:%S.%3f%z").to_string(),
        "comment": {
            "content": [
              {
                "content": [
                  {
                    "text": worklog.description,
                    "type": "text"
                  }
                ],
                "type": "paragraph"
              }
            ],
            "type": "doc",
            "version": 1
          },
    });

    let response = client
        .post(url)
        .basic_auth(user, Some(api_token))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&payload)
        .send();

    match response {
        Ok(resp) if resp.status() == 201 => 
            Ok(()),
        Ok(resp) => 
            Err(format!("Update failed with status: {}", resp.status()).into()),  
        Err(err) => 
            Err(format!("Request failed with error: {}", err).into())
    }
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