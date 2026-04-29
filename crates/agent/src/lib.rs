use anyhow::Result;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct AgentService {
    system_prompt: String,
}

impl AgentService {
    pub fn new() -> Self {
        Self {
            system_prompt: "You are a reliable Rust LLM assistant. Prefer tool usage for arithmetic and retrieval tasks.".to_string(),
        }
    }

    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    pub async fn run_tools_if_needed(&self, prompt: &str) -> Result<Option<String>> {
        if let Some((x, y)) = parse_add(prompt) {
            debug!("tool router selected add tool");
            let out = AddTool.call(OperationArgs { x, y }).await?;
            return Ok(Some(format!("Tool result: {out}")));
        }

        if let Some((x, y)) = parse_subtract(prompt) {
            debug!("tool router selected subtract tool");
            let out = SubtractTool.call(OperationArgs { x, y }).await?;
            return Ok(Some(format!("Tool result: {out}")));
        }

        Ok(None)
    }
}

impl Default for AgentService {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_add(input: &str) -> Option<(i32, i32)> {
    let parts: Vec<i32> = input
        .split_whitespace()
        .filter_map(|token| token.parse::<i32>().ok())
        .collect();
    if input.to_lowercase().contains("add") && parts.len() >= 2 {
        Some((parts[0], parts[1]))
    } else {
        None
    }
}

fn parse_subtract(input: &str) -> Option<(i32, i32)> {
    let parts: Vec<i32> = input
        .split_whitespace()
        .filter_map(|token| token.parse::<i32>().ok())
        .collect();
    if input.to_lowercase().contains("subtract") && parts.len() >= 2 {
        Some((parts[0], parts[1]))
    } else {
        None
    }
}

#[derive(Debug, Deserialize)]
pub struct OperationArgs {
    x: i32,
    y: i32,
}

#[derive(Debug, Error)]
#[error("math tool failed")]
pub struct MathToolError;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AddTool;

impl Tool for AddTool {
    const NAME: &'static str = "add";
    type Error = MathToolError;
    type Args = OperationArgs;
    type Output = i32;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Add x and y together".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "x": { "type": "number" },
                    "y": { "type": "number" }
                },
                "required": ["x", "y"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(args.x + args.y)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SubtractTool;

impl Tool for SubtractTool {
    const NAME: &'static str = "subtract";
    type Error = MathToolError;
    type Args = OperationArgs;
    type Output = i32;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Subtract y from x".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "x": { "type": "number" },
                    "y": { "type": "number" }
                },
                "required": ["x", "y"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(args.x - args.y)
    }
}

#[cfg(test)]
mod tests {
    use super::AgentService;

    #[tokio::test]
    async fn routes_add_tool() {
        let service = AgentService::new();
        let out = service
            .run_tools_if_needed("please add 2 and 5")
            .await
            .expect("tool execution should succeed");
        assert_eq!(out.as_deref(), Some("Tool result: 7"));
    }
}
