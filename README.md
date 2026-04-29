# Rust LLM Agent Platform MVP

Rust LLM agent backend with local OSS model support, typed tool orchestration, persistence, and observability.

## Stack
- `axum` + `tokio` for HTTP and async runtime.
- `rig` (`rig-core`) for tool abstractions in agent workflows.
- Ollama local model runtime via HTTP API.
- `sqlx` + SQLite for session/run persistence.
- `tracing` + `tower-http` for structured logs and request traces.

## Quick Start
1. Copy `.env.example` to `.env` and adjust values.
2. Start with Docker Compose:
   - `docker compose -f ops/docker/docker-compose.yml up --build`
3. Pull a model in Ollama container if needed:
   - `docker exec -it <ollama-container> ollama pull llama3.1:8b-instruct-q4_K_M`

## Local Dev
- `cargo check`
- `cargo test`
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets -- -D warnings`

## API
- `GET /health`
- `POST /api/v1/chat`
  - Body: `{ "prompt": "add 2 3" }`
- `GET /api/v1/runs/{limit}`

## Notes
- Debug-level logs are enabled by default for development; reduce `RUST_LOG` for production.
- Tool routing currently includes arithmetic tools and can be extended via `crates/agent`.
