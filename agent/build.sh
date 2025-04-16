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
    *)
      echo "Unknown option: $1"
      shift
      ;;
  esac
done

# Use environment variables if arguments not provided
if [ -z "$TARGET" ]; then
    TARGET=${TARGET:-"x86_64-unknown-linux-gnu"}
fi

if [ -z "$OUTPUT_DIR" ]; then
    SERVER_IP=${LISTENER_HOST:-$(ip route get 1 | awk '{print $7;exit}')}
    SERVER_PORT=${LISTENER_PORT:-8080}
    OUTPUT_DIR="../server/static/agents"
else
    SERVER_IP=${LISTENER_HOST:-$(ip route get 1 | awk '{print $7;exit}')}
    SERVER_PORT=${LISTENER_PORT:-8080}
fi

echo "Configuration:"
echo "  Target:       $TARGET"
echo "  Output Dir:   $OUTPUT_DIR"
echo "  Build Type:   $BUILD_TYPE"
echo "  C2 Server:    ${SERVER_IP}:${SERVER_PORT}"
echo "  Sleep:        ${SLEEP_INTERVAL:-60} seconds"
echo "  Jitter:       ${JITTER:-2} seconds"

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

echo "Building agent..."

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# Create config file
cat > "$OUTPUT_DIR/config.json" << EOF
{
    "server_url": "${SERVER_IP}:${SERVER_PORT}",
    "sleep_interval": ${SLEEP_INTERVAL:-60},
    "jitter": ${JITTER:-2}
}
EOF

echo "Created configuration in $OUTPUT_DIR/config.json"

# Determine build command based on target
if [ "$BUILD_TYPE" == "debug" ]; then
    BUILD_FLAGS=""
else
    BUILD_FLAGS="--release"
fi

# Build the agent
echo "Building for $TARGET..."
if [ "$TARGET" == "x86_64-pc-windows-gnu" ]; then
    # Windows build
    cargo build $BUILD_FLAGS --target $TARGET
    BINARY_EXT=".exe"
else
    # Linux build
    cargo build $BUILD_FLAGS --target $TARGET
    BINARY_EXT=""
fi

# Determine the build directory
if [ "$BUILD_TYPE" == "debug" ]; then
    BUILD_DIR="target/$TARGET/debug"
else
    BUILD_DIR="target/$TARGET/release"
fi

# Copy the binary to the output directory
cp "$BUILD_DIR/agent$BINARY_EXT" "$OUTPUT_DIR/agent$BINARY_EXT"
cp "$BUILD_DIR/agent$BINARY_EXT" "$OUTPUT_DIR/$(date +%Y%m%d%H%M%S)_agent$BINARY_EXT"

echo "Build complete!"
echo "Agent saved to: $OUTPUT_DIR/agent$BINARY_EXT"
echo "Config saved to: $OUTPUT_DIR/config.json"
