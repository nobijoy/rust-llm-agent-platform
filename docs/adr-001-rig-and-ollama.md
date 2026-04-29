# ADR-001: Rig + Ollama for MVP

## Status
Accepted

## Context
The MVP must ship in 2-3 weeks and showcase senior-level Rust architecture with local OSS model support and minimal infrastructure overhead.

## Decision
Use:
- `rig` for typed tool abstractions and agent composition.
- Ollama for local model serving through a stable HTTP API.
- A clean internal gateway trait so model providers can be swapped later.

## Consequences
- Faster time-to-demo than custom inference pipelines.
- Strong architecture story: provider abstraction, tool contracts, and testable layers.
- Future upgrade path to vLLM or additional providers without API surface changes.
