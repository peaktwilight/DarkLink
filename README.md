This project proposes the design of a lightweight C2 framework for penetration testing and red team operations, emphasizing stealth and a low system footprint. The framework will utilize covert communication, encryption, and obfuscation to evade detection by security tools like EDR and IDS. Its effectiveness will be evaluated in simulated environments, offering insights into evasion techniques and improving defensive strategies.

---

## Getting Started

### Prerequisites
- **Go** (v1.20+ recommended)
- **Rust** (for agent builds)
- **Node.js & npm** (for web UI development, if you plan to modify frontend)
- **Git** (to clone the repository)

### Installation
1. **Clone the repository:**
   ```sh
   git clone https://github.com/yourusername/MicroC2.git
   cd MicroC2
   ```
2. **Build the server:**
   ```sh
   cd server
   go build -o server ./cmd/server.go
   ```
3. **Build the agent (optionally use cargo strip to reduce compile build as much as possible):**
   ```sh
   cd ../agent
   cargo build --release
   ```

### Running the Server
1. **Start the server:**
   ```sh
   cd server
   ./server
   ```
2. **Access the web interface:**
   - Open your browser and go to: [http://localhost:8080](http://localhost:8080) (or the port you configured).

### Configuration
- Edit `server/config/settings.yaml` for server settings.
- Edit `agent/src/config.rs` or use environment variables for agent configuration.

### Creating Listeners
- Use the web UI to create HTTP Polling or SOCKS5 listeners.
- Agents will connect to the listener endpoints you configure.

### Building Payloads
- Use the Payload Generator in the web UI to generate agent binaries for your target OS/architecture.

### File Drop
    - Upload and download files via the File Drop section in the web UI. Folder in codebase is MicroC2/server/uploads/

---

