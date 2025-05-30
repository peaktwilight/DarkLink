use std::fs;
use std::error::Error;
use reqwest::Client;
use obfstr::obfstr;

/// Uploads a file to the given URL via HTTP POST using reqwest.
pub async fn upload_file_to_url(file_path: &str, upload_url: &str) -> Result<String, Box<dyn Error>> {
    let file_content = fs::read(file_path)?;
    
    let client = Client::new();
    let response = client
        .post(upload_url)
        .header("Content-Type", "application/octet-stream")
        .body(file_content)
        .send()
        .await?;
    
    if response.status().is_success() {
        Ok(format!("File uploaded successfully: {}", response.status()))
    } else {
        Err(format!("Upload failed with status: {}", response.status()).into())
    }
}
