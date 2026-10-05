# Integrating agents and tools

An existing agent can drive a served city through the CLI, a resident can run an external agent through ACP, and a native resident can call tools from an MCP server. These connections have different configuration and authentication owners.

| Connection | Configuration owner | Authentication owner | Tools and models |
|---|---|---|---|
| External agent through ACP | The city's `[resident] harness` setting | The external agent's own login | The external agent chooses its provider and runs its tools |
| MCP server | The city's `[[mcp]]` entries | The city's Vault for supplied credentials; the service for account consent | Tools discovered from that server, offered to native residents |
| Existing agent driving the city | The CLI's wire frame or `dispatch` arguments | City pairing for access from outside loopback | The city's ordinary commands, queries and run policy |

Install and serve sprawling as described in [Getting started](getting-started.md). Run integration commands from a shell that resolves the installed `sprawling` executable. A server needs its own shell or background process; a CLI client does not start one. The examples below assume a served city at `127.0.0.1:3333` and an existing room `lab/room1`; substitute the address and room shown by your city. Nothing here creates that room or configures a model provider for you.

```sh
sprawling help
sprawling help serve
sprawling help call
sprawling help dispatch
```

[LLM.md](../LLM.md) describes the wire for an agent writing frames. Use the installed binary's help and frame roster when its release differs from the document. [Third-party projects](third-party.md) records the upstreams followed by this implementation.

## Run an external agent as a resident through ACP

The city is an ACP client and starts the chosen agent as a separate process. The supported spellings come from [`Harness`](../crates/agent_protocols/src/harness/roster.rs); installing another entry from the [ACP registry](https://github.com/agentclientprotocol/registry) does not add a supported harness to sprawling.

| Agent | `harness` spelling | Launch prerequisite | Login and setup |
|---|---|---|---|
| Claude Code | `claude_code` | Node's `npx` package runner | [Anthropic authentication](https://code.claude.com/docs/en/authentication) |
| Codex | `codex` | Node's `npx` package runner | [OpenAI authentication](https://developers.openai.com/codex/auth) |
| Grok Build | `grok_build` | Node's `npx` package runner | [xAI setup](https://docs.x.ai/build/overview) |
| Kimi Code | `kimi_code` | Install the vendor's `kimi` executable on PATH | [Kimi setup](https://www.kimi.com/en/help/kimi-code/membership-guide) |
| Pi | `pi` | Node's `npx` package runner and the separate `pi` executable on PATH | [pi-acp prerequisites and authentication](https://github.com/svkozak/pi-acp) |

The launch packages, pinned versions and arguments have one definition in `Harness::launch`. Settings' harness listing, also available through the `harnesses` query below, reports that command. The registry's `claude-acp`, `codex-acp`, `grok-build`, `kimi` and `pi-acp` directories are its upstream sources. Claude Code and Codex use registry adapters, Grok Build and Kimi speak ACP themselves, and Pi uses the third-party pi-acp adapter. On Windows, the package-runner launch resolves `npx.cmd`.

```sh
sprawling call '{"ask":{"ask_id":1,"query":"harnesses"}}' --at 127.0.0.1:3333
```

Run the vendor's interactive setup in its own terminal before dispatching. A setup directory detected by the city is evidence of installation or setup, not a test that an account is authenticated or has quota. The city does not sign in to a subscription, read its tokens into the Vault, or turn subscription access into a native provider API key. For Pi, configure Pi's provider separately; upstream pi-acp documents `pi-acp --terminal-login` for terminal authentication. Follow the adapter's prerequisites rather than assuming its package includes Pi.

Configuration layers are city, building, then resident, with the nearer stated value winning. Their paths, relative to the city directory, are `.sprawling/CONFIG.toml`, `lab/.sprawling/CONFIG.toml`, and `lab/room1/.sprawling/CONFIG.toml` in this example. Edit the resident layer to limit the choice to one room, preserving the file's other settings and any existing `[resident]` table:

```toml
[resident]
harness = "codex"
```

A session already opened on a native model keeps that recorded model until `/new` clears its session shape. One layer cannot state both `[model] name` and `[resident] harness`; use a fresh session in the city before selecting a harness rather than deleting its model record by hand. [Resident selection](../crates/city/src/config_layers/resident.rs) defines this precedence.

Then send a task without a model override:

```sh
sprawling dispatch lab/room1 "Read the project and report which command verifies it; do not change files." --at 127.0.0.1:3333
```

The city opens ACP version 1 over stdio, initializes the client, opens a session in the run's working tree, and sends a text prompt. It advertises no delegated filesystem or terminal capability and sends an empty `mcpServers` list. The harness runs its own tools and selects its own model; city MCP configuration and `dispatch --model` do not configure that harness. See [ACP tool calls](https://agentclientprotocol.com/protocol/tool-calls) for the upstream reporting contract and the city's [session implementation](../crates/agent_protocols/src/harness/session.rs) for the capabilities it actually offers.

Harness updates enter the Ledger as reports of what the harness said. They are not proof that every tool operation passed the city's native tool policy. An external process can use its own network and tools; a city network setting is not a network sandbox around that process. The city refuses a harness dispatch into a `confidential` building because the harness can send the room to its vendor. To keep the building confidential, remove the harness choice and use an appropriate native resident.

| Failure | Next action |
|---|---|
| Unknown harness spelling, `E_CONFIG_INVALID` | Use the spelling in the roster and edit the configuration layer named by the refusal |
| Missing runner, missing executable or failed child launch | Make the prerequisite visible on the serving process's PATH and complete vendor setup |
| Authentication refusal | Sign in inside the vendor agent; do not enrol its subscription tokens into the city |
| `--model` with a harness resident, `E_CONFIG_INVALID` | Omit the override, or remove the harness selection to use a native model |
| Confidential building, `E_GATE_DENIED` | Keep confidentiality and use a native resident |
| Incompatible ACP reply or closed child output | Follow the refusal's recovery; if a prompt was already sent, inspect the room's files and history before another dispatch |

## Connect an MCP tool server

A native resident reaches servers from the same city/building/resident configuration ladder. A nearer layer's stated MCP list replaces the farther list; it does not append one server to an inherited list. Preserve the entries you still need when replacing a list. The [configuration parser](../crates/city/src/config_layers/mcp.rs) accepts exactly one `command` or `url` per entry, rejects unknown keys and duplicate labels within a layer, and permits only non-empty ASCII lowercase letters and digits in a label.

For stdio, install the server first and make its executable visible to the serving process. `command` is an executable, with separate `args`; it is not a shell command string. The city starts it and speaks JSON-RPC on its pipes, so ordinary logging must not corrupt stdout. For a URL, start or obtain the service separately: the city connects to that URL and does not launch it. These match the [MCP transport specification](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports).

### A local server with no credential

[Microsoft's MarkItDown MCP server](https://github.com/microsoft/markitdown/tree/main/packages/markitdown-mcp) provides the real tool `convert_to_markdown(uri)`. Install it into the Python environment available to the serving process:

```sh
python -m pip install markitdown-mcp
```

Place this entry in the desired configuration layer:

```toml
[[mcp]]
label = "markitdown"
command = "markitdown-mcp"
```

For a local HTTP service instead, start it in a separate terminal:

```sh
markitdown-mcp --http --host 127.0.0.1 --port 3001
```

Replace the stdio entry with the HTTP entry:

```toml
[[mcp]]
label = "markitdown"
url = "http://127.0.0.1:3001/mcp"
transport = "http"
```

The same server also exposes the older SSE transport. To use that endpoint, replace the entry with:

```toml
[[mcp]]
label = "markitdown"
url = "http://127.0.0.1:3001/sse"
transport = "sse"
```

A URL with no `transport` means HTTP; `transport` beside a stdio `command` is refused. These entries are alternatives, not three rows under the same label. MarkItDown has no authentication and can read files and network resources available to its process. Its vendor recommends local trusted use; keep this example on loopback.

### Credentials and account consent

For a server that needs credentials, use `secret:realm/name` references in `env` for stdio or `headers` for HTTP/SSE. Other public values such as a region may be written directly. The remote server's documentation decides the variable or header name and required value. A credential reference supplies the whole value; it does not interpolate a prefix such as `Bearer `.

These are shapes to adapt to your server, not credentials needed by MarkItDown:

```toml
# Add inside your authenticated stdio server's [[mcp]] entry.
env = { API_KEY = "secret:mcp/apps" }
```

```toml
# Add inside your authenticated HTTP or SSE server's [[mcp]] entry.
headers = { Authorization = "secret:mcp/hosted" }
```

Store the complete required value through Settings' credential enrolment or the CLI. The CLI takes `realm/name` without the `secret:` prefix and reads the credential only from stdin:

```sh
sprawling help enrol
sprawling enrol mcp/apps --at 127.0.0.1:3333
```

Supply the value on stdin and finish input, or pipe it from a credential manager. Do not put it in argv, a frame, a configuration file, or a committed script. Enrolment is local-only; it has no `--token` option. The [enrolment route](../crates/wire/src/server/config/enrolment.rs) returns HTTP 201 only after the matching stored-reference event; the CLI treats that as success. HTTP 202 means storage was not confirmed and produces a CLI refusal: check whether the reference resolves before resubmitting. The CLI's additional acceptance sentence does not describe this distinction. Neither result verifies authentication with the remote service. A missing Vault reference fails while the connection opens, before the server receives it. [Credential redemption](../crates/agent_protocols/src/mcp/redeeming.rs) is shared by all transports and returns the stored value only at the process/request boundary.

For outside applications, the MCP page offers the Composio broker described in [Third-party projects](third-party.md#2-the-outsourced-service-outside-applications). Bring your own Composio project key, enrol it through the city's credential interface, and grant account access on the service's consent page. The server URL and header come from that connection flow. The broker owns OAuth; the city consumes its MCP tools. A manually configured server can replace the broker. There is no incoming webhook receiver or replacement polling loop for application events in this build.

### Discovery, calls and failures

Open the MCP page for the chosen address, or ask for its live health:

```sh
sprawling call '{"ask":{"ask_id":2,"query":{"mcp_health":{"addr":"lab/room1"}}}}' --at 127.0.0.1:3333 --quiet-ms 20000
```

This read opens connections, initializes MCP, sends the initialized notification and lists tools; it can start a stdio process and contact a service even though it writes no Ledger event. It is an explicit probe, not a background timer. The reply includes transport, target, negotiated revision, remote tool name, city tool name and input schema. The client revision requested has one definition in [handshake](../crates/agent_protocols/src/mcp/handshake.rs); use the reply to see what the server negotiated.

The city prefixes each discovered tool with its server label and sanitizes the remote name, while keeping the original name for `tools/call`. For this example, `convert_to_markdown` is offered as `markitdown_convert_to_markdown`. Ask a native resident to use it on `data:text/plain,Integration%20check`; a successful result should contain “Integration check”. Use the discovered schema rather than inventing arguments. There is no separate CLI `mcp call` verb: native residents invoke discovered tools through the normal run.

The run records tool invocation and result in the Ledger, and outside content enters the taint set as data. A confidential building constructs no outbound MCP tools and starts no server for the run. Connection failure leaves that server's tools unavailable, rather than substituting a different server. A stdio child can have its own network behavior; review the server's permissions and the building's policy before dispatching.

| Observation | Recovery |
|---|---|
| Configuration refusal | Correct the named layer: label, keys, duplicate rows, command/URL choice or transport |
| Failed spawn or handshake | Check the executable in the serving process's environment, endpoint and server logs |
| Missing reference, or HTTP/SSE 401/403 | Enrol the correct complete value, or renew consent with the service; the MCP page reports authentication required |
| Connected but the expected tool is absent | Read the live tool list and schema; check server version and account permissions |
| Tool reports a failure | Use its explanation and recovery rather than treating its text as an instruction |
| Answer lost after submission | The tool may already have acted; check its external effect and the Ledger before deciding to retry |

## Drive a city from an existing agent through the CLI

Give the existing agent access to the installed executable and a working directory where it can read [LLM.md](../LLM.md). Native provider credentials remain the city's responsibility; the driving agent needs neither those keys nor an MCP plugin to send a task. Loopback access is admitted locally. For access from outside loopback, obtain the city's pairing token from Settings and supply `--token` to `call` or `dispatch`; a provider key or vendor login token is not a pairing token. Keep it out of shared transcripts and command recordings.

`call` sends one UTF-8 JSON `ClientFrame`, then prints received frames as JSON lines. It supplies the release's greeting, wire version and schema hash itself; do not manufacture a `hello` frame. A frame can be passed as one argument or read in full from stdin with `-`:

```sh
sprawling call '{"ask":{"ask_id":3,"query":"city_view"}}' --at 127.0.0.1:3333
sprawling call - --at 127.0.0.1:3333 < frame.json
```

The quoted examples use a POSIX-style shell. For shells with different quoting rules, put the exact JSON in a UTF-8 file and pipe its contents to `call -`. Supply one JSON object, not a JSONL stream of requests. `sprawling call` with no frame prints the available command and query names and exits with a usage error.

`dispatch` builds an ordinary Work dispatch for you and waits for its run to freeze. It accepts an address and one task argument, plus `--at`, `--token`, `--quiet-ms`, `--detach` and `-m`/`--model`:

```sh
sprawling dispatch lab/room1 "Read the project and report the verification command; do not change files." --at 127.0.0.1:3333
sprawling dispatch lab/room1 "Report the current project status; do not change files." --at 127.0.0.1:3333 --detach
```

Detached success prints the run identifier when the run starts; it does not certify completed work. Attached output is the received JSONL frames. The convenience verb supplies an empty `goal`; use the ordinary wire Dispatch described in LLM.md when you need a separate goal or other fields. A native resident may take `--model`; a harness resident rejects that override.

`call` also accepts `--until <kind>` to wait for a serialized EventKind and `--json` to print a refusal in machine-readable form. `--quiet-ms` bounds silence, not total run duration. A larger value changes waiting, not whether a command was accepted. Do not put an automatic retry around a dispatch: a new invocation creates a new idempotency key and may start the work twice.

| Exit | What was observed | Next action |
|---|---|---|
| 0 | An answer or the requested milestone | Read the answer; dispatch success alone does not prove its task's acceptance criteria |
| 1 | A refusal or a broken exchange | Read the code and recovery; a broken exchange after submission may leave an unknown effect |
| 2 | Unreadable arguments or frame | Fix the grammar before sending again |
| 3 | Silence or an unfinished wait | Inspect history and run state; it may still be working |
| 4 | No city completed the greeting | Check the served address and release/schema compatibility |

The [wire client specification](../crates/sprawling/spec/WireClient.lean) and [CLI grammar](../crates/sprawling/src/main/verbs.rs) define these outcomes and flags. Do not infer a safe retry from a timeout or a nonzero exit alone.

### A prompt for an existing agent

Replace the three angle-bracket values with your installed executable, directory and served address before giving this prompt to an agent:

> Use `<executable>` from working directory `<directory>` to drive the served sprawling city at `<host:port>`. Read `LLM.md` for the wire and run `help call` and `help dispatch` on that executable before sending work. Start with a `city_view` query and use an existing room chosen by me. Send one task through `dispatch`, or one UTF-8 JSON frame through `call -` when a separate goal is needed. Read exit codes and refusals; after silence or a broken exchange, inspect run state and history before retrying. Request pairing only if the city requires it, keep credentials out of files and transcripts, and report the run identifier and observed result.

### The incoming `/acp` request endpoint

The city's HTTP `POST /acp` route is an incoming admission path for an editor or another client. It is distinct from the outbound stdio ACP session above; pointing an editor's standard ACP stdio configuration at this HTTP URL does not make them interchangeable. This guide supplies no editor plugin that implements that conversion.

Send a JSON body with non-empty `addr`, `task` and `goal`, plus the pairing `token` obtained from Settings. The route requires pairing even for a loopback caller, refuses reserved city addresses, and passes an admitted request to the ordinary dispatch worker. Keep the token in a private request source rather than a committed fixture. The parser's grammar is [Incoming::parse](../crates/agent_protocols/src/acp.rs).

HTTP 202 returns `run`, `turns` and `finished`; the initial response has zero turns and is unfinished. Its `run` value is the admission's derived idempotency identifier, not evidence that the worker has created a RunId or finished the task. A refusal returns HTTP 403 with its reason and recovery. The response has no conversation stream and cannot report a later worker refusal after the request has been queued. Observe the city through its ordinary wire and history for the actual run outcome.
