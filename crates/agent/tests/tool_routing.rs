use agent::{AgentService, ToolContext};

#[tokio::test]
async fn routes_add_tool() {
    let service = AgentService::new();
    let out = service
        .run_tools_if_needed("please add 2 and 5", &ToolContext::default())
        .await
        .expect("tool execution should succeed")
        .expect("tool response expected");
    assert!(out.contains("\"tool_name\":\"add\""));
    assert!(out.contains("\"value\":7"));
}

#[tokio::test]
async fn routes_calculator_tool() {
    let service = AgentService::new();
    let out = service
        .run_tools_if_needed("calc 2 + 3 * 4", &ToolContext::default())
        .await
        .expect("calculator should succeed")
        .expect("tool response expected");
    assert!(out.contains("\"tool_name\":\"calculator\""));
    assert!(out.contains("\"value\":14"));
}
