use std::fs;
use std::path::Path;
use std::error::Error;
use hyper::{Client, Request, Body};
use hyper::header::{CONTENT_TYPE, CONTENT_LENGTH};
use bytes::Bytes;

pub async fn upload_file(url: &str, filepath: &str) -> Result<(), Box<dyn Error>> {
    let content = fs::read(filepath)?;
    let filename = Path::new(filepath)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(filepath);

    let client = Client::new();
    
    let req = Request::builder()
        .method("POST")
        .uri(url)
        .header("X-Filename", filename)
        .header(CONTENT_TYPE, "application/octet-stream")
        .header(CONTENT_LENGTH, content.len())
        .body(Body::from(content))?;

    let resp = client.request(req).await?;
    
    if !resp.status().is_success() {
        return Err("Upload failed".into());
    }
    
    Ok(())
}
