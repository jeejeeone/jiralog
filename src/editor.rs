use std::error::Error;
use std::fs::OpenOptions;
use std::io::{self, Write, Read};
use std::process::{Command, Stdio};


pub fn start_editor(initial_content: &str, editor_command: &str) -> Result<String, Box<dyn Error>> {
    let mut child = Command::new(editor_command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.as_mut() {
        writeln!(stdin, "{}", initial_content)?;
    }

    let output = child.wait_with_output()?;

    let mut content = String::new();
    let stdout = output.stdout;
    content.push_str(&String::from_utf8_lossy(&stdout));

    Ok(content)
}

pub fn start_editor2() -> Result<String, Box<dyn Error>> {
    // Define a temporary file path.
    let file_path = "/tmp/nano_temp_file.txt";

    // Create or overwrite the file with the initial content.
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_path)?;
        writeln!(file, "zap")?;
    }

    // Open `nano` with the file.
    let status = Command::new("nano")
        .arg(file_path)
        .status()?;

    if !status.success() {
        eprintln!("Failed to edit the file with nano");
        return Ok("heoo".to_string());
    }

    // Read the content of the file after editing.
    let content = std::fs::read_to_string(file_path)?;

    // Print the content of the file after editing.
    println!("Content of the file after editing:\n{}", content);

    // Optionally, clean up the temporary file.
    std::fs::remove_file(file_path)?;

    Ok("heoo".to_string())
}