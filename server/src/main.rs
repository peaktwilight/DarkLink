mod test_server;

#[tokio::main]
async fn main() {
    println!("Starting MicroC2 Server...");
    test_server::run_test_server().await;
}
