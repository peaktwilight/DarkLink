use std::fs;
use std::error::Error;
use hyper::{Client, Request};
use hyper::Body;
use hyper::header::{CONTENT_TYPE, CONTENT_LENGTH};

/// Uploads a file to the given URL via HTTP POST.
/// The file is sent with 'application/octet-stream' content type and
/// an 'X-Filename' header set to the base file name. Returns an error
/// if file reading or network operations fail.
pub async fn upload_file(url: &str, filepath: &str) -> Result<(), Box<dyn Error>> {
    let content = fs::read(filepath)?;
    let filename = std::path::Path::new(filepath)
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
