use std::error::Error;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde::Serialize;

pub fn run_editor<T: Serialize>(content: Vec<&T>, editor_command: &str, temp_file_path: &PathBuf) -> Result<Vec<String>, Box<dyn Error>> {
    let commit_edit_file = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .truncate(true)
        .open(temp_file_path)?;
        
    let mut writer = csv::WriterBuilder::new().from_writer(commit_edit_file);
    content.iter().try_for_each(|v| writer.serialize(v))?;
    writer.flush()?;
    
    let status = Command::new(editor_command)
        .arg(temp_file_path)
        .status()?;

    if !status.success() {
        return Err(format!("Editor exited with {}", status.code().unwrap_or(1)).into());
    }

    let after_edit_commit_file = OpenOptions::new()
        .read(true)
        .open(temp_file_path)?;

    let reader = BufReader::new(after_edit_commit_file);
    let lines = reader.lines().map(|v|v.unwrap()).collect();

    std::fs::remove_file(temp_file_path)?;

    Ok(lines)
}