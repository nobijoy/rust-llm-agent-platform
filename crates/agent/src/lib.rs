use anyhow::Result;
use reqwest::Client;
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

    pub async fn run_tools_if_needed(
        &self,
        prompt: &str,
        context: &ToolContext,
    ) -> Result<Option<String>> {
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

        if let Some(expression) = parse_expression(prompt) {
            debug!("tool router selected calculator tool");
            let out = evaluate_expression(&expression)?;
            return Ok(Some(format!("Tool result: {out}")));
        }

        if should_use_memory_tool(prompt) {
            debug!("tool router selected memory tool");
            return Ok(Some(memory_summary(context)));
        }

        if prompt.to_ascii_lowercase().contains("latest node version") {
            debug!("tool router selected latest-node-version web tool");
            let msg = fetch_latest_node_version().await;
            return Ok(Some(msg));
        }

        if let Some(url) = parse_fetch_url(prompt) {
            debug!("tool router selected web fetch tool");
            let msg = fetch_url_excerpt(&url).await;
            return Ok(Some(msg));
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

#[derive(Debug, Clone)]
pub struct MemoryRecord {
    pub created_at: String,
    pub user_prompt: String,
    pub response: String,
}

#[derive(Debug, Clone, Default)]
pub struct ToolContext {
    pub memory: Vec<MemoryRecord>,
}

fn should_use_memory_tool(prompt: &str) -> bool {
    let lowered = prompt.to_ascii_lowercase();
    lowered.contains("what did i ask")
        || lowered.contains("recent prompts")
        || lowered.contains("memory")
}

fn memory_summary(context: &ToolContext) -> String {
    if context.memory.is_empty() {
        return "Memory tool: no stored runs yet.".to_string();
    }

    let lines = context
        .memory
        .iter()
        .take(5)
        .enumerate()
        .map(|(idx, run)| {
            format!(
                "{}. [{}] prompt='{}' response='{}'",
                idx + 1,
                run.created_at,
                truncate(&run.user_prompt, 80),
                truncate(&run.response, 80)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("Memory tool (latest runs):\n{lines}")
}

fn parse_fetch_url(prompt: &str) -> Option<String> {
    let trimmed = prompt.trim();
    let lowered = trimmed.to_ascii_lowercase();
    if lowered.starts_with("fetch ") {
        return trimmed
            .split_whitespace()
            .nth(1)
            .map(std::string::ToString::to_string);
    }
    None
}

async fn fetch_url_excerpt(url: &str) -> String {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return "Web fetch tool error: URL must start with http:// or https://".to_string();
    }

    let client = Client::new();
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.text().await {
            Ok(text) => format!("Web fetch tool result:\n{}", truncate(&text, 600)),
            Err(err) => format!("Web fetch tool error: failed reading body: {err}"),
        },
        Ok(resp) => format!("Web fetch tool error: non-success status {}", resp.status()),
        Err(err) => format!("Web fetch tool error: {err}"),
    }
}

async fn fetch_latest_node_version() -> String {
    #[derive(Debug, Deserialize)]
    struct NodeRelease {
        version: String,
    }

    let client = Client::new();
    let req = client.get("https://nodejs.org/dist/index.json");
    match req.send().await {
        Ok(resp) if resp.status().is_success() => match resp.json::<Vec<NodeRelease>>().await {
            Ok(releases) => match releases.first() {
                Some(latest) => format!(
                    "Web fetch tool result: latest Node.js version is {} (source: https://nodejs.org/dist/index.json)",
                    latest.version
                ),
                None => "Web fetch tool error: no releases found in Node.js index.".to_string(),
            },
            Err(err) => format!("Web fetch tool error: invalid Node.js index payload: {err}"),
        },
        Ok(resp) => format!(
            "Web fetch tool error: Node.js index status {}",
            resp.status()
        ),
        Err(err) => format!("Web fetch tool error: {err}"),
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (idx, ch) in value.chars().enumerate() {
        if idx >= max_chars {
            out.push_str("...");
            return out;
        }
        out.push(ch);
    }
    out
}

fn parse_expression(prompt: &str) -> Option<String> {
    let lowered = prompt.to_ascii_lowercase();
    let expr = if lowered.starts_with("calc ") {
        prompt[5..].trim()
    } else if lowered.starts_with("calculate ") {
        prompt[10..].trim()
    } else {
        return None;
    };

    if expr.is_empty() {
        None
    } else {
        Some(expr.to_string())
    }
}

fn evaluate_expression(expression: &str) -> Result<f64> {
    let tokens = tokenize(expression)?;
    let rpn = to_rpn(&tokens)?;
    eval_rpn(&rpn)
}

#[derive(Debug, Clone, Copy)]
enum Token {
    Number(f64),
    Op(char),
    LParen,
    RParen,
}

fn tokenize(expr: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = expr.chars().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch.is_ascii_digit() || ch == '.' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let num = expr[start..i].parse::<f64>()?;
            out.push(Token::Number(num));
            continue;
        }
        match ch {
            '+' | '-' | '*' | '/' => out.push(Token::Op(ch)),
            '(' => out.push(Token::LParen),
            ')' => out.push(Token::RParen),
            _ => return Err(anyhow::anyhow!("unsupported token: '{ch}'")),
        }
        i += 1;
    }
    Ok(out)
}

fn precedence(op: char) -> u8 {
    match op {
        '+' | '-' => 1,
        '*' | '/' => 2,
        _ => 0,
    }
}

fn to_rpn(tokens: &[Token]) -> Result<Vec<Token>> {
    let mut output = Vec::new();
    let mut ops: Vec<Token> = Vec::new();

    for token in tokens {
        match token {
            Token::Number(_) => output.push(*token),
            Token::Op(op) => {
                while let Some(Token::Op(top)) = ops.last().copied() {
                    if precedence(top) >= precedence(*op) {
                        output.push(ops.pop().expect("checked is_some"));
                    } else {
                        break;
                    }
                }
                ops.push(*token);
            }
            Token::LParen => ops.push(*token),
            Token::RParen => {
                while let Some(top) = ops.pop() {
                    match top {
                        Token::LParen => break,
                        other => output.push(other),
                    }
                }
            }
        }
    }

    while let Some(token) = ops.pop() {
        if matches!(token, Token::LParen | Token::RParen) {
            return Err(anyhow::anyhow!("mismatched parentheses"));
        }
        output.push(token);
    }
    Ok(output)
}

fn eval_rpn(tokens: &[Token]) -> Result<f64> {
    let mut stack: Vec<f64> = Vec::new();
    for token in tokens {
        match token {
            Token::Number(n) => stack.push(*n),
            Token::Op(op) => {
                let rhs = stack
                    .pop()
                    .ok_or_else(|| anyhow::anyhow!("invalid expression"))?;
                let lhs = stack
                    .pop()
                    .ok_or_else(|| anyhow::anyhow!("invalid expression"))?;
                let result = match op {
                    '+' => lhs + rhs,
                    '-' => lhs - rhs,
                    '*' => lhs * rhs,
                    '/' => lhs / rhs,
                    _ => return Err(anyhow::anyhow!("unsupported operator '{op}'")),
                };
                stack.push(result);
            }
            Token::LParen | Token::RParen => {
                return Err(anyhow::anyhow!("invalid expression token"));
            }
        }
    }
    if stack.len() != 1 {
        return Err(anyhow::anyhow!("invalid expression"));
    }
    Ok(stack[0])
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
    use super::{AgentService, ToolContext};

    #[tokio::test]
    async fn routes_add_tool() {
        let service = AgentService::new();
        let out = service
            .run_tools_if_needed("please add 2 and 5", &ToolContext::default())
            .await
            .expect("tool execution should succeed");
        assert_eq!(out.as_deref(), Some("Tool result: 7"));
    }

    #[tokio::test]
    async fn routes_calculator_tool() {
        let service = AgentService::new();
        let out = service
            .run_tools_if_needed("calc 2 + 3 * 4", &ToolContext::default())
            .await
            .expect("calculator should succeed");
        assert_eq!(out.as_deref(), Some("Tool result: 14"));
    }
}
