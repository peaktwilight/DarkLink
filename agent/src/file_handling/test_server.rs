use std::net::SocketAddr;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use std::fs;

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/download/test.txt") => {
            match fs::read("received_test.txt") {
                Ok(content) => {
                    println!("File downloaded successfully!");
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "text/plain")
                        .body(Body::from(content))
                        .unwrap())
                },
                Err(e) => {
                    eprintln!("Failed to read file: {}", e);
                    Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::from("File not found"))
                        .unwrap())
                }
            }
        },
        (&Method::POST, "/upload") => {
            let body_bytes = hyper::body::to_bytes(req.into_body())
                .await
                .unwrap_or_default();
            
            // Save uploaded content to a file
            if let Err(e) = fs::write("received_test.txt", &body_bytes) {
                eprintln!("Failed to save file: {}", e);
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Upload failed"))
                    .unwrap());
            }
            
            println!("File uploaded successfully!");
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from("Upload successful"))
                .unwrap())
        },
        _ => {
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not Found"))
                .unwrap())
        }
    }
}

pub async fn run_test_server() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(handle_request))
    });

    let server = Server::bind(&addr).serve(make_svc);
    println!("Test server running on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_test_server().await;
    Ok(())
}
