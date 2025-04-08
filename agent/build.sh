#!/bin/bash

echo "Checking dependencies..."

# Install required packages if not present
if ! dpkg -l | grep -q "gcc-mingw-w64"; then
    echo "Installing gcc-mingw-w64..."
    sudo apt-get install -y gcc-mingw-w64
fi

# Install cross if not already installed
if ! cargo install --list | grep -q "^cross"; then
    echo "Installing cross..."
    cargo install cross
fi

echo "Building agents..."

# Get server IP
SERVER_IP=$(ip route get 1 | awk '{print $7;exit}')
SERVER_PORT=${PORT:-8080}

# Create config files
create_config() {
    cat > "target/$1/release/config.json" << EOF
{
    "server_url": "${SERVER_IP}:${SERVER_PORT}",
    "sleep_interval": 5,
    "jitter": 2
}
EOF
}

# Create Windows package
create_windows_package() {
    echo "Creating Windows package..."
    PACKAGE_DIR="../server/static/agents/windows_package"
    mkdir -p "$PACKAGE_DIR"
    
    # Copy agent and config
    cp "target/x86_64-pc-windows-gnu/release/agent.exe" "$PACKAGE_DIR/agent.exe"
    cp "target/x86_64-pc-windows-gnu/release/config.json" "$PACKAGE_DIR/config.json"
    
    # Create run script
    cat > "$PACKAGE_DIR/run.bat" << EOF
@echo off
echo Starting MicroC2 Agent...
agent.exe
pause
EOF

    # Create ZIP file
    cd ../server/static/agents
    zip -r windows_agent.zip windows_package/*
    cd ../../../agent
    
    echo "Windows package created at /server/static/agents/windows_agent.zip"
}

# Build for Windows
echo "Building for x86_64-pc-windows-gnu..."
cargo build --release --target x86_64-pc-windows-gnu
create_config "x86_64-pc-windows-gnu"
create_windows_package

# Build for Linux
echo "Building for x86_64-unknown-linux-gnu..."
cargo build --release --target x86_64-unknown-linux-gnu
create_config "x86_64-unknown-linux-gnu"

# Copy binaries and configs to output directory
mkdir -p "../server/static/agents"
cp "target/x86_64-pc-windows-gnu/release/agent.exe" "../server/static/agents/agent_windows_amd64.exe"
cp "target/x86_64-pc-windows-gnu/release/config.json" "../server/static/agents/config_windows.json"
cp "target/x86_64-unknown-linux-gnu/release/agent" "../server/static/agents/agent_linux_amd64"
cp "target/x86_64-unknown-linux-gnu/release/config.json" "../server/static/agents/config_linux.json"

echo "Build complete!"
