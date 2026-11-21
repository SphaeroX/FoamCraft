// SPDX-License-Identifier: GPL-3.0-or-later
//! The MCP protocol side: tools/list, tools/call, the guide as a resource, server instructions.

use std::future::Future;
use std::sync::Arc;

use base64::Engine as _;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ListResourcesResult, ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResponse, ReadResourceResult, Resource, ResourceContents, ServerCapabilities,
    ServerConfig, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::Value;

use crate::{GUIDE, GUIDE_URI, INSTRUCTIONS, Output, Server, error_json};

#[derive(Clone)]
pub struct McpHandler {
    server: Arc<Server>,
}

impl McpHandler {
    pub fn new(server: Arc<Server>) -> Self {
        Self { server }
    }
}

fn content(out: Vec<Output>) -> Vec<ContentBlock> {
    out.into_iter()
        .map(|o| match o {
            Output::Json(v) => ContentBlock::text(v.to_string()),
            Output::Png(bytes) => ContentBlock::image(
                base64::engine::general_purpose::STANDARD.encode(bytes),
                "image/png",
            ),
        })
        .collect()
}

impl ServerHandler for McpHandler {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new("foamcraft", foamcraft_core::VERSION))
        .with_instructions(INSTRUCTIONS)
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_ {
        let tools = self
            .server
            .tools()
            .into_iter()
            .map(|t| {
                let schema = match t.input_schema {
                    Value::Object(map) => map,
                    _ => Default::default(),
                };
                Tool::new(t.name, t.description, Arc::new(schema))
            })
            .collect();
        std::future::ready(Ok(ListToolsResult::with_all_items(tools)))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResponse, ErrorData>> + Send + '_ {
        let server = self.server.clone();
        async move {
            let name = request.name.to_string();
            let args = request.arguments.map(Value::Object).unwrap_or(Value::Null);
            // Commands block (meshing, waiting); keep them off the protocol threads.
            let result = tokio::task::spawn_blocking(move || server.call(&name, args))
                .await
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            Ok(match result {
                Ok(out) => CallToolResult::success(content(out)),
                Err(e) => {
                    CallToolResult::error(vec![ContentBlock::text(error_json(&e).to_string())])
                }
            }
            .into())
        }
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, ErrorData>> + Send + '_ {
        let guide = Resource::new(GUIDE_URI, "guide")
            .with_title("How to set up and check a FoamCraft simulation")
            .with_mime_type("text/markdown");
        std::future::ready(Ok(ListResourcesResult::with_all_items(vec![guide])))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResponse, ErrorData>> + Send + '_ {
        std::future::ready(if request.uri == GUIDE_URI {
            Ok(ReadResourceResult::new(vec![ResourceContents::text(GUIDE, GUIDE_URI)]).into())
        } else {
            Err(ErrorData::resource_not_found(request.uri, None))
        })
    }
}
