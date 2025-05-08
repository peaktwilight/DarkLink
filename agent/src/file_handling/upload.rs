use std::fs;
use std::error::Error;
use hyper::{Client, Request};
use hyper::Body;
use hyper::header::{CONTENT_TYPE, CONTENT_LENGTH};
use obfstr::obfstr;

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
        .header(obfstr!("X-Filename").to_string(), filename)
        .header(CONTENT_TYPE, obfstr!("application/octet-stream").to_string())
        .header(CONTENT_LENGTH, content.len())
        .body(Body::from(content))?;

    let resp = client.request(req).await?;
    
    if !resp.status().is_success() {
        return Err(obfstr!("Upload failed").to_string().into());
    }
    
    Ok(())
}
