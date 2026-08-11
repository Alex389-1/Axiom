# Axiom - Fast Local AI Agent

Axiom is a powerful, lightning-fast local AI coding assistant and desktop application built with **Tauri v2**, **React**, **TypeScript**, and **Rust**. 

**Core Idea:** Axiom lets fast, non-thinking local models use tools through an external **Agent Runtime**, instead of requiring the model itself to support native reasoning or tool calling. The model doesn't need to be an agent — the runtime makes it an agent.

![Axiom](public/tauri.svg)

## How It Works

Existing agent GUIs assume the local model supports native tool calling, structured function calls, and agentic planning. This often fails with smaller or faster models.

Axiom solves this by moving the agent logic out of the model and into a dedicated **Rust-based Agent Runtime**:
```mermaid
graph TD
    %% Main Components
    GUI["Tauri GUI Client"]
    Daemon["Rust Agent Daemon (Runtime)"]
    
    %% GUI communicates with Daemon
    GUI <-->|"IPC / WebSockets"| Daemon
    
    %% Daemon Sub-components
    subgraph Runtime ["Agent Runtime"]
        Planner["Planner & Router"]
        Context["Context Manager"]
        Perms["Permissions Manager"]
    end
    Daemon --- Runtime
    
    %% External Interfaces
    subgraph Targets ["Execution Targets"]
        LLM["Local LLM (Ollama/llama.cpp)"]
        PTY["Persistent Terminal (PTY)"]
        FS["Filesystem & Git"]
    end
    
    %% Connections
    Planner --> LLM
    Planner --> PTY
    Planner --> FS
    Context -.->|"Reads Context"| FS
    Perms -.->|"Gates Actions"| PTY
```

### 1. Tool-Calling Reliability Layer
For smaller models (e.g. 7B/14B parameters), Axiom uses a robust two-step reliability layer:
- **Constrained Generation:** Uses GBNF grammars or JSON-schema mode at the provider level to force valid tool schema generation.
- **Repair/Retry Parser:** If constraints aren't available, Axiom parses free text natively, catching errors and re-prompting the model automatically on failures before falling back to ReAct-style extraction.

### 2. Persistent Terminal & Background Daemon
Axiom never owns the shell process directly in the GUI. Instead, a **PTY Manager** in the background daemon maintains your working directory, environment, shell history, and running processes (like `npm run dev` or `cargo test`). The terminal survives GUI panel closes and restarts.

### 3. Session-Scoped Permissions
Axiom implements a smart permission system to prevent fatigue during iterative loops. You can grant **Allow for Session** or **Allow for Project** permissions to specific tool categories (e.g. `EXECUTE`). High-risk commands (`sudo`, `rm -rf`) bypass caching and will always explicitly prompt for permission.

### 4. Smart Context Management
Axiom does not blindly dump your repository into the context window. It uses a deterministic keyword and `ripgrep`-based retrieval strategy to find exact matches across your project, alongside your recent terminal outputs, git diffs, and project manifests—strictly capped by a token budget.

## Technology Stack

Axiom is built for maximum efficiency and stability:
- **Frontend**: Tauri v2, React 19, TypeScript, Vite, Vanilla CSS.
- **Backend (Daemon)**: Rust, Tokio, portable-pty, SQLite.
- **LLM Support**: Ollama and llama.cpp (OpenAI-compatible).

## Supported Platforms

Axiom is built with cross-platform technologies and is designed to run locally on your machine.

- **Linux (Primary Target)**: Fully supported. Tested on Arch Linux, Ubuntu, Debian, and Fedora.
- **macOS**: Fully supported.
- **Windows**: Supported. The core stack is fully compatible, though you may need minor manual configuration adjustments (e.g., using Named Pipes instead of Unix sockets, and configuring `powershell.exe` as the default shell).

---

## Installation & Setup

### Quick Setup (One-Command)

If you are on Linux or macOS, you can run our all-in-one setup script. It will automatically check for Node.js, install Rust, install necessary system dependencies, and download the Node packages:

```bash
chmod +x install.sh
./install.sh
```

### Manual Installation

If you prefer to set up manually:

**1. Install Prerequisites**
Ensure you have [Rust](https://www.rust-lang.org/tools/install), [Node.js](https://nodejs.org/en/) (v18+), and [Ollama](https://ollama.com/) installed.

**2. Clone & Install**
```bash
git clone https://github.com/Alex389-1/Axiom.git
cd Axiom
npm install
```

**3. Run the Development Server**
```bash
npm run tauri dev
```

**4. Build for Production**
```bash
npm run tauri build
```
Compiled binaries will be available in `src-tauri/target/release/bundle/`.

---

## Uninstallation

To completely remove Axiom, its Node dependencies, and Rust build artifacts, you can run the provided uninstall script:

```bash
chmod +x uninstall.sh
./uninstall.sh
```

---

## License

This project is licensed under the MIT License.
