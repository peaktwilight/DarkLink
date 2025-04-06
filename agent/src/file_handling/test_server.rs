use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use std::path::Path;
use std::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

const UPLOAD_DIR: &str = "uploaded_files";

fn generate_file_list_html() -> String {
    let mut html = String::from(r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>MicroC2 File Drop Test Server</title>
            <style>
                body { font-family: Arial, sans-serif; margin: 40px; }
                .file { padding: 10px; border-bottom: 1px solid #eee; }
                .file:hover { background: #f5f5f5; }
                .download-link { float: right; }
            </style>
        </head>
        <body>
            <h1>MicroC2 Test Server</h1>
            <h2>Uploaded Files:</h2>
    "#);

    if let Ok(entries) = std::fs::read_dir(UPLOAD_DIR) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Ok(filename) = entry.file_name().into_string() {
                    html.push_str(&format!(r#"
                        <div class="file">
                            {} 
                            <a class="download-link" href="/download/{}">Download</a>
                        </div>
                    "#, filename, filename));
                }
            }
        }
    }

    html.push_str("</body></html>");
    html
}

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            Ok(Response::new(Body::from(generate_file_list_html())))
        },
        (&Method::POST, "/upload") => {
            handle_upload(req).await
        }
        (&Method::GET, path) if path.starts_with("/download/") => {
            handle_download(path).await
        }
        _ => {
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("404 Not Found"))
                .unwrap())
        }
    }
}

async fn handle_upload(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    if !Path::new(UPLOAD_DIR).exists() {
        fs::create_dir_all(UPLOAD_DIR).unwrap();
    }

    // Extract filename from headers before consuming the request
    let filename = if let Some(name) = req.headers().get("X-Filename") {
        format!("{}/{}_{}", UPLOAD_DIR,
            chrono::Local::now().format("%Y%m%d_%H%M%S"),
            name.to_str().unwrap_or("uploaded_file"))
    } else {
        format!("{}/uploaded_file_{}", UPLOAD_DIR,
            chrono::Local::now().format("%Y%m%d_%H%M%S"))
    };

    // Now consume the request body
    let body_bytes = hyper::body::to_bytes(req.into_body())
        .await
        .unwrap();

    let mut file = File::create(&filename).await.unwrap();
    file.write_all(&body_bytes).await.unwrap();

    Ok(Response::new(Body::from(format!("File uploaded as: {}", 
        Path::new(&filename).file_name().unwrap().to_string_lossy()))))
}

async fn handle_download(path: &str) -> Result<Response<Body>, Infallible> {
    let filename = path.strip_prefix("/download/").unwrap();
    let filepath = format!("{}/{}", UPLOAD_DIR, filename);

    match fs::read(&filepath) {
        Ok(contents) => Ok(Response::new(Body::from(contents))),
        Err(_) => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("File not found"))
            .unwrap())
    }
}

pub async fn run_test_server() {
    let addr = ([127, 0, 0, 1], 8080).into();

    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(handle_request))
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("Test server running on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_server() {
        run_test_server().await;
    }
}
