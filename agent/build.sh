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

# Make OUTPUT_DIR absolute path to avoid issues
if [[ "$OUTPUT_DIR" != /* ]]; then
    OUTPUT_DIR="$(pwd)/$OUTPUT_DIR"
fi

echo "Configuration:"
echo "  Target:       $TARGET"
echo "  Output Dir:   $OUTPUT_DIR"
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
            cross build $BUILD_FLAGS $CARGO_FEATURES --target $TARGET
        else
            cross build $BUILD_FLAGS --target $TARGET
        fi
    else
        echo "Cross tool not found, using direct cargo build..."
        # Make sure the Windows target is installed
        rustup target add $TARGET
        if [ "$FORMAT" == "windows_dll" ]; then
            echo "Building with DLL features: cargo build $BUILD_FLAGS $CARGO_FEATURES --target $TARGET"
            cargo build $BUILD_FLAGS $CARGO_FEATURES --target $TARGET
        else
            cargo build $BUILD_FLAGS --target $TARGET
        fi
    fi
else
    # Linux build
    cargo build $BUILD_FLAGS --target $TARGET
fi

# Determine the build directory
if [ "$BUILD_TYPE" == "debug" ]; then
    BUILD_DIR="target/$TARGET/debug"
else
    BUILD_DIR="target/$TARGET/release"
fi

echo "Build complete in $BUILD_DIR"

# Copy the binary to the output directory
if [ -f "$BUILD_DIR/agent$BINARY_EXT" ]; then
    echo "Copying agent binary to output directory"
    cp "$BUILD_DIR/agent$BINARY_EXT" "$OUTPUT_DIR/agent$BINARY_EXT"
    cp "$BUILD_DIR/agent$BINARY_EXT" "$OUTPUT_DIR/$(date +%Y%m%d%H%M%S)_agent$BINARY_EXT"
else
    echo "WARNING: agent binary not found at expected location: $BUILD_DIR/agent$BINARY_EXT"
    # List directory contents to aid debugging
    echo "Contents of $BUILD_DIR:"
    ls -la "$BUILD_DIR"
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
                cp "$BUILD_DIR/agent$BINARY_EXT" "$OUTPUT_DIR/agent.dll"
                echo "Copied DLL from $BUILD_DIR/agent$BINARY_EXT to $OUTPUT_DIR/agent.dll"
            else
                # Fallback to explicit DLL creation with gcc if needed
                echo "Creating DLL from executable using gcc"
                x86_64-w64-mingw32-gcc -shared -o "$OUTPUT_DIR/agent.dll" \
                    -Wl,--out-implib="$OUTPUT_DIR/libagent.a" \
                    -Wl,--export-all-symbols \
                    -Wl,--enable-auto-import \
                    "$BUILD_DIR/agent$BINARY_EXT"
            fi
            
            # Verify DLL exports
            if command -v x86_64-w64-mingw32-objdump &> /dev/null; then
                echo "Verifying DLL exports in $OUTPUT_DIR/agent.dll"
                x86_64-w64-mingw32-objdump -p "$OUTPUT_DIR/agent.dll" | grep -A15 "Export Table"
            fi
            
            # Make sure the DLL file exists
            if [ -f "$OUTPUT_DIR/agent.dll" ]; then
                echo "DLL file created successfully at: $OUTPUT_DIR/agent.dll"
                # Display file size and permissions
                ls -la "$OUTPUT_DIR/agent.dll"
            else
                echo "ERROR: Failed to create DLL file at $OUTPUT_DIR/agent.dll"
            fi
        else
            echo "ERROR: Agent binary not found at $BUILD_DIR/agent$BINARY_EXT"
            exit 1
        fi
    else
        echo "ERROR: Cannot create Windows DLL for non-Windows target: $TARGET"
        exit 1
    fi
fi

echo "Build complete!"
echo "Agent saved to: $OUTPUT_DIR/agent$BINARY_EXT"
if [ "$FORMAT" == "windows_dll" ]; then
    echo "DLL saved to: $OUTPUT_DIR/agent.dll"
fi
echo "Config saved to: $OUTPUT_DIR/config.json"
