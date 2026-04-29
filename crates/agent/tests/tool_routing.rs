use agent::{AgentService, ToolContext};

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
