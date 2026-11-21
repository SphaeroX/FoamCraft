// SPDX-License-Identifier: GPL-3.0-or-later
//! The MCP protocol with a test client over an in-memory pipe: initialize with instructions,
//! tools/list with valid schemas, a tool call, and the guide as a resource.

use std::sync::Arc;

use foamcraft_mcp::{GUIDE_URI, McpHandler, Server};
use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, ReadResourceRequestParams, ResourceContents};

#[tokio::test]
async fn a_client_lists_the_tools_calls_one_and_reads_the_guide() {
    let dir = std::env::temp_dir().join(format!("fc-mcp-protocol-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let server = Server::local(
        foamcraft_api::registry(),
        dir.join("data"),
        vec![dir.join("work")],
    );
    let (client_io, server_io) = tokio::io::duplex(1 << 20);
    let handler = McpHandler::new(Arc::new(server));
    tokio::spawn(async move {
        let running = handler.serve(tokio::io::split(server_io)).await.unwrap();
        let _ = running.waiting().await;
    });
    let client = ().serve(tokio::io::split(client_io)).await.unwrap();

    let info = client.peer_info().unwrap();
    assert!(
        info.instructions
            .as_deref()
            .unwrap_or("")
            .contains("FoamCraft")
    );
    let tools = client.list_all_tools().await.unwrap();
    assert!(tools.len() >= 19);
    for t in &tools {
        assert_eq!(
            t.input_schema.get("type").and_then(|v| v.as_str()),
            Some("object"),
            "{}",
            t.name
        );
        assert!(t.description.as_deref().is_some_and(|d| !d.is_empty()));
    }

    let args = serde_json::json!({"path": dir.join("work/p"), "name": "Via MCP"});
    let out = client
        .call_tool(
            CallToolRequestParams::new("project_new")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(out.is_error, Some(false));
    let text = serde_json::to_string(&out.content).unwrap();
    assert!(text.contains("Via MCP"), "{text}");

    // A refused path comes back as a tool error with its code, not as a protocol error.
    let args = serde_json::json!({"path": std::env::temp_dir().join("fc-elsewhere"), "name": "X"});
    let out = client
        .call_tool(
            CallToolRequestParams::new("project_new")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(out.is_error, Some(true));
    assert!(
        serde_json::to_string(&out.content)
            .unwrap()
            .contains("mcp.path_outside_roots")
    );

    let resources = client.list_all_resources().await.unwrap();
    assert!(resources.iter().any(|r| r.uri == GUIDE_URI));
    let guide = client
        .read_resource(ReadResourceRequestParams::new(GUIDE_URI))
        .await
        .unwrap();
    let ResourceContents::TextResourceContents { text, .. } = &guide.contents[0] else {
        panic!("text expected")
    };
    assert!(text.contains("run_wait"));
    client.cancel().await.unwrap();
}
