use std::fs;
use std::io;
use std::path::Path;

pub async fn upload_file(url: &str, filepath: &str) -> io::Result<()> {
    let content = fs::read(filepath)?;
    let filename = Path::new(filepath)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(filepath);

    reqwest::Client::new()
        .post(url)
        .header("X-Filename", filename)
        .body(content)
        .send()
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(())
}
