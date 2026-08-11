# Axiom - Fast Local AI Agent

Axiom is a powerful, lightning-fast local AI coding assistant and desktop application built with **Tauri v2**, **React**, **TypeScript**, and **Rust**. Axiom connects to your local LLMs (like Ollama and Llama.cpp) to provide an integrated agentic workspace directly on your machine.

![Axiom](public/tauri.svg)

## Features

- **Local LLM Integration**: Connects seamlessly with local inference engines (Ollama, Llama.cpp).
- **Agentic Workflows**: Includes built-in agent features, timeline visualizations, and background daemon support.
- **Premium UI/UX**: Dark-themed, highly responsive interface with dynamic layouts, sleek custom markdown rendering, and collapsible panels.
- **Code Focused**: Native code block styling, built-in "Copy" buttons, language tags, and seamless file attachment processing.
- **Voice Integration**: Built-in voice dictation (speech-to-text) and read-aloud functionality.
- **Customizable**: Control Model settings, System Prompts, Temperature, and Max Tokens easily.
- **Privacy First**: Everything runs locally on your machine. No cloud, no telemetry.

## Technology Stack

- **Frontend**: React 19, TypeScript, Vite, Vanilla CSS
- **Backend / Daemon**: Rust, Tauri v2, Tokio
- **Icons & Markdown**: Lucide React, React Markdown

---

## Installation & Setup

### Prerequisites

Ensure you have the following installed on your system:
- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/en/) (v18 or higher)
- [Ollama](https://ollama.com/) (For local LLM inference)

### 1. Clone the Repository

### Quick Setup (One-Command)

If you are on Linux or macOS, you can run our all-in-one setup script. It will automatically check for Node.js, install Rust, install necessary system dependencies, and download the Node packages:

```bash
chmod +x install.sh
./install.sh
```

### Manual Installation

If you prefer to set up manually:

**1. Install Dependencies**
Install the Node dependencies for the frontend:

```bash
npm install
```

### 3. Start the Development Server

To start the application in development mode (which spins up both the Vite frontend server and the Tauri Rust backend):

```bash
npm run tauri dev
```

### 4. Build for Production

When you are ready to package the app into a standalone executable (AppImage, deb, dmg, or exe depending on your OS):

```bash
npm run tauri build
```

The compiled binaries will be located in the `src-tauri/target/release/bundle/` directory.

---

## Contributing

Contributions, issues, and feature requests are welcome! Feel free to check the [issues page](https://github.com/Alex389-1/Axiom/issues).

## License

This project is licensed under the MIT License.
