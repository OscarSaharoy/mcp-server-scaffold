# HTTP MCP Server Tutorial

For some reason its hard to find a good MCP server tutorial :( most are AI generated

This repo contains steps to create a remote HTTP MCP server.


## Checking your MCP server works

It helps to check the MCP server is working as you develop it - this can be done with the [MCP Inspector tool](https://github.com/modelcontextprotocol/inspector).

You can just run `npx @modelcontextprotocol/inspector` and then it gives you a UI to test your server as you go.


## Which SDK to use

I think the rust SDK is best, just because there are API docs available (https://docs.rs/rmcp/0.4.1/rmcp/). To use it you will need to [install rust](https://www.rust-lang.org/tools/install).


## How to deploy it

For a remote MCP server (using the HTTP transport rather than stdio) you will need to host it with a URL accessible to the web so that people can send it requests.

You can do it however like, on an EC2 instance or similar or even self hosted, but I think [Vercel functions](https://vercel.com/docs/functions) is optimal for easiest development, lowest cost, and most mature rust support. So for local development you will need to install the [vercel CLI](https://vercel.com/docs/cli) (`npm i -g vercel`).

> [!NOTE]
> I found that this stack can be hard to work with if you have a slow network connection as there is a lot to download and build - sorry if this doesn't work for that reason.


## Getting started

This is how to create the minimal code to run an MCP server over HTTP with the rust SDK:

### 1. Create `Cargo.toml`

You can use these commands to create a minimal `Cargo.toml` and add dependencies:

The `[[bin]]` section is needed for the vercel function to work correctly.

```bash
cargo init
cat << EOF >> Cargo.toml
tokio = { version = "1.47.1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
rmcp = { version = "0.2.0", features = ["server", "transport-streamable-http-server-session", "transport-sse-server", "transport-worker", "transport-streamable-http-server"] }
vercel_runtime = { version = "1" }
tower = "0.5.2"
http-body-util = "0.1.3"

[[bin]]
name = "mcp"
path = "api/mcp.rs"
EOF
```

You can test that your base rust project is working by running `cargo run` - after it compiles it should print "Hello, World!" :)


### 2. Create `vercel.json` and `.vercelignore`

`vercel.json` is the file that configures vercel to be able to serve your MCP tools :) this is specifying that the file at `api/mcp.rs` is a vercel function, and it will be available at the `/api/mcp` path when we deploy to the web (it will be available at `http://localhost:3000/api/mcp` during development when you run `vercel dev`).

```bash
cat << EOF > vercel.json
{
  "functions": {
    "api/mcp.rs": {
      "runtime": "vercel-rust@4.0.9"
    }
  }
}
EOF
```

`.vercelignore` just tells vercel not to upload the rust build artefacts as they can be very large.

```bash
echo "target/" > .vercelignore
```


### 3. Add MCP server code to `api/mcp.rs`

Now create your main rust file with minimal code to run the server - the minimal code I landed on is about 100 lines, hopefully its clear enough :)

The code will just be in one file at `api/mcp.rs`, as I believe the code needs to be in the `api` directory rather than `src` for vercel to make a serverless function from it.

```bash
mv src/ api/
mv api/main.rs api/mcp.rs

cat << EOF > api/mcp.rs
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
EOF
```


### 4. Run locally with `vercel dev`

To test your MCP server locally, run `vercel dev` which should start a local development server. It might ask you some setup questions to help initialise your vercel project at this point; you should be able to accept all the defaults.

When the server has started and you request the url that corresponds to your serverless function (`http://localhost:3000/api/mcp`), the code will build which might take a moment.

```bash
(venv) [oscarsaharoy@LCCC-MB-pTIY21 ~/projects/mcp-server-scaffold] $ vercel dev
Vercel CLI 44.3.0
> Ready! Available at http://localhost:3000
```

If you just visit the URL in your browser its not going to work as your browser doesn't use MCP. So you can use the [MCP Inspector tool](https://github.com/modelcontextprotocol/inspector) by running `npx @modelcontextprotocol/inspector`. Then, in the UI that it gives you, enter `http://localhost:3000/api/mcp` as the URL and "Streamable HTTP" as the transport. When you click Connect, if should work nicely, and let you list the tools available and run the test tool which should return a test message.

You can make any edits to the code that you would like to, and test them from the MCP inspector.


### 5. Deploy to vercel

Now to deploy your MCP server to production, run `vercel`. It will show you the build logs as it deploys, unfortunately it can take a few minutes. But once it completes your MCP server should be deployed and usable :) You can try to connect to it with the MCP inspector by entering the url (in my case it is `https://mcp-server-scaffold.vercel.app/api/mcp` as I called the vercel project "mcp-server-scaffold").

