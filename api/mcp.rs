use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager, StreamableHttpServerConfig,
};
use vercel_runtime::{run, Body, Error, Request, Response};
use http_body_util::BodyExt;
use tower::util::ServiceExt;
use rmcp::{
    model::ErrorData as McpError,
    RoleServer,
    ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    schemars,
    service::RequestContext,
    tool, tool_handler, tool_router,
};

#[derive(Clone)]
pub struct MyMCPServerHandler {
    tool_router: ToolRouter<MyMCPServerHandler>,
    // counter: Arc<Mutex<i32>>,  // to persist data between tool calls you can add fields something like this
}

// This defines the input parameters for the `get_test_message` tool
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TestMessageParams {
    pub test_param: String,
    // pub test_param_2: i32,  // you can add more params like this
}

#[tool_router]
impl MyMCPServerHandler {

    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Return a test string")]
    async fn get_test_message(
        &self,
        Parameters(TestMessageParams { test_param }): Parameters<TestMessageParams>
    ) -> Result<CallToolResult, McpError> {

        Ok(CallToolResult::success(vec![Content::text(
            format!("Hello World! Value of test_param is: {}", test_param),
        )]))
    }
}

#[tool_handler]
impl ServerHandler for MyMCPServerHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()  // this enables MCP tools. You can also add eg. `.enable_prompts()` and `.enable_resources()`
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This is a test MCP server. Use the `get_test_message` tool to return a test message.".to_string()
            ),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,  // the _request and _context parameters contain info about the request that can be parsed to get eg. the request URI and headers
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {

        Ok(self.get_info())
    }
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {

    let service = StreamableHttpService::new(
        || Ok(MyMCPServerHandler::new()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig {
            sse_keep_alive: None, // Not using SSE in serverless
            stateful_mode: false, // Stateless fits serverless best
        },
    );

    let response = service
        .oneshot(req)
        .await?;

    // Convert the body from BoxBody<Bytes, Infallible> to Vercel Body
    let (parts, body) = response.into_parts();
    let bytes = body.collect().await?.to_bytes();
    let vercel_body = Body::Binary(bytes.to_vec());

    Ok(Response::from_parts(parts, vercel_body))}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}
