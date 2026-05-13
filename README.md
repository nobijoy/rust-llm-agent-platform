# Rust LLM Agent Platform

A high-performance, modular backend for LLM-powered agents. This platform integrates local Large Language Models (LLMs) with a typed tool orchestration layer, persistence, and observability, all built with Rust's safety and speed.

## 🚀 Overview

The Rust LLM Agent Platform provides a robust foundation for building intelligent agents that can:
- Interact with local OSS models via **Ollama**.
- Execute complex tasks using a **modular tool system**.
- Maintain state and context through **SQLite persistence**.
- Provide a high-concurrency **REST API** for chat and observability.

## 🏗️ Architecture

The project is structured as a Rust workspace with specialized crates for better separation of concerns:

- **`crates/api`**: The entry point. An `axum` web server handling HTTP requests, routing, and state management.
- **`crates/agent`**: Core orchestration logic. Routes user prompts to specific tools or falls back to the LLM. Implements the `AgentService` and various tool definitions.
- **`crates/model`**: Gateway layer for LLM providers. Currently supports **Ollama** via the `ModelGateway` trait.
- **`crates/storage`**: Data persistence layer using `sqlx` and SQLite. Manages session history and agent runs.
- **`crates/common`**: Shared utilities, configuration management (`AppConfig`), and centralized tracing initialization.

## 🛠️ Tool System

The platform features a sophisticated tool routing mechanism. Before hitting the LLM, the `AgentService` attempts to resolve prompts using specialized tools:

- **Math & Arithmetic**:
  - `AddTool`: Simple binary addition.
  - `SubtractTool`: Simple binary subtraction.
  - `Calculator`: A robust expression evaluator using Reverse Polish Notation (RPN) for complex math (e.g., `calc (2 + 3) * 4`).
- **Web Capabilities**:
  - `Web Fetch`: Fetches and summarizes content from a given URL.
  - `Node.js Version`: Retrieves the latest stable Node.js version directly from the official distribution index.
- **Memory & Context**:
  - `Memory Retrieval`: Accesses recent session history to provide context-aware responses (triggered by prompts like "what did I just ask?").

## 🚦 Getting Started

### Prerequisites
- [Rust](https://www.rust-lang.org/) (2024 edition)
- [Ollama](https://ollama.com/) (running locally or in a container)
- [Docker](https://www.docker.com/) (optional, for containerized deployment)

### Configuration
Environment variables can be set in a `.env` file:
| Variable | Description | Default |
|----------|-------------|---------|
| `APP_HOST` | Server bind host | `0.0.0.0` |
| `APP_PORT` | Server bind port | `8080` |
| `OLLAMA_BASE_URL` | URL for Ollama API | `http://127.0.0.1:11434` |
| `OLLAMA_MODEL` | LLM model name | `llama3.1:8b-instruct-q4_K_M` |
| `DATABASE_URL` | SQLite connection string | `sqlite://./data/agent.db` |
| `RUST_LOG` | Logging level | `info,api=debug,agent=debug` |

### Quick Start
1. **Setup Environment**:
   ```bash
   cp .env.example .env
   ```
2. **Launch with Docker**:
   ```bash
   docker compose -f ops/docker/docker-compose.yml up --build
   ```
3. **Run Locally**:
   ```bash
   cargo run -p api
   ```

## 📡 API Endpoints

### `POST /api/v1/chat`
The main interaction point.
- **Request Body**: `{ "prompt": "calculate (10 + 5) / 3" }`
- **Response**: A JSON object containing the `session_id` and the agent's `response`. If a tool was used, the response will be a structured JSON string with tool metadata.

### `GET /api/v1/runs/{limit}`
Retrieve the most recent agent runs for observability.
- **Path Parameter**: `limit` (max 100)
- **Returns**: A list of recent interactions including timestamps, prompts, and responses.

### `GET /health`
Standard health check endpoint.

## 🧪 Testing & Quality
- **Unit Tests**: `cargo test` runs the workspace-wide test suite.
- **Linting**: `cargo clippy` is used for static analysis.
- **Formatting**: `cargo fmt` ensures consistent style.
- **Tracing**: Structured JSON logging is enabled via `tracing-subscriber` for production-grade observability.

## 📝 License
MIT
