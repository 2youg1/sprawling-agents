# Integrating agents and tools

An existing agent can drive a served city through the CLI, any agent that speaks ACP can be a resident, and a native resident can call tools from an MCP server. These connections have different configuration and authentication owners.

| Connection | Configuration owner | Authentication owner | Tools and models |
|---|---|---|---|
| External agent through ACP | The city's `[[agent]]` rows and the room's `[resident] harness` setting | The external agent's own login, run by its own program | The external agent chooses its provider and runs its tools |
| MCP server | The city's `[[mcp]]` entries | The city's Vault for supplied credentials; the service for account consent | Tools discovered from that server, offered to native residents |
| Existing agent driving the city | The CLI's wire frame or `dispatch` arguments | The key file on this machine, or city pairing from outside loopback | The city's ordinary commands, queries and run policy |

Install and serve sprawling as described in [Getting started](getting-started.md). Run integration commands from a shell that resolves the installed `sprawling` executable. A server needs its own shell or background process; a CLI client does not start one. The examples below assume a served city at `127.0.0.1:3333` and an existing room `lab/room1`; substitute the address and room shown by your city. Nothing here creates that room or configures a model provider for you.

```sh
sprawling help
sprawling help serve
sprawling help call
sprawling help dispatch
```

[the wire reference](wire.md) describes the wire for an agent writing frames. Use the installed binary's help and frame roster when its release differs from the document. [Third-party projects](third-party.md) records the upstreams followed by this implementation.

## Run any ACP agent as a resident

The city is an ACP client: any agent that speaks ACP version 1 over stdio can be a resident, and the city starts it as a separate process. There are three ways in, and every one ends at the same consent card. <!-- v0.0.11-verify -->

| Way in | What you do | What the city knows before you consent |
|---|---|---|
| Found on this machine | open the ACP page, or `/acp` in the CLI; agents found on this machine are listed first | the command it would run and the evidence it found, read without running anything |
| The bundled catalog | type two or three letters of its name | the entry of the [ACP registry](https://github.com/agentclientprotocol/registry) as of the snapshot this release ships, with its pinned version |
| Pasted | paste one command line, a Zed or JetBrains `agent_servers` block, or a registry `agent.json` into the same box, or after `/acp` | what that text says, parsed by the city; shell syntax such as `\|`, `&`, `;`, `<` and `>` is refused rather than interpreted |

The consent card shows the exact command line, the version and whether it is pinned, where the entry came from, the licence, the names of the environment variables it sets, and that the agent runs with your account's rights, including over this city. **Add and use here** adds the agent and seats it in the room you are in; **add only** adds it for later. Consent binds what the card showed: the city recomputes the digest of that launch specification and refuses if it changed, for example after the catalog was refreshed. Nothing of the agent runs before consent, not even a probe. Adding is local-only: a device paired through the remote door cannot add an agent or start a login. <!-- v0.0.11-verify -->

An added agent is one `[[agent]]` row in the city's `.sprawling/CONFIG.toml`, written for you; a row can also be written by hand:

```toml
[[agent]]
id = "gemini"
name = "Gemini CLI"
command = "C:/Program Files/nodejs/npx.cmd"
args = ["-y", "@google/gemini-cli@0.63.0", "--acp"]
source = "registry"
version = "0.63.0"
```

`command` is an absolute path, resolved when you consent, and the agent starts without a shell. `env` may set variables, and a credential there is a `secret:realm/name` reference; a plaintext credential in `env` is refused when the file is read. `source` is `registry`, `detected` or `pasted`. The city records `launch_digest` itself at consent. [Configuration layers](../crates/city/spec/ConfigLayers.lean) define the grammar and its refusals. <!-- v0.0.11-verify -->

Seating chooses who answers in a room. Configuration layers are city, building, then resident, with the nearer stated value winning; each layer uses `.sprawling/CONFIG.toml` relative to its own directory. **Add and use here** writes the resident layer of that room; by hand, preserve the file's other settings and any existing `[resident]` table:

```toml
[resident]
harness = "gemini"
```

The value names an `[[agent]]` row or one of the built-in catalog entries, which include the five official harnesses: Claude Code, Codex, Grok Build, Kimi Code and Pi. A session already opened on a native model keeps that recorded model until `/new` clears its session shape, so seating an agent in such a room opens a new session, and the button says so. One layer cannot state both `[model] name` and `[resident] harness`. [Resident selection](../crates/city/src/config_layers/resident.rs) defines this precedence. <!-- v0.0.11-verify -->

Then send a task without a model override:

```sh
sprawling dispatch lab/room1 "Read the project and report which command verifies it; do not change files." --at 127.0.0.1:3333
```

**Logging in.** The agent signs in when it first says it must, not when it is added. When it answers a session or a prompt with ACP's authentication-required error, the city offers the login methods the agent declared: in the CLI, as numbered choices that hand the terminal to the agent's own login program and take it back; on the page, as buttons that start that program in a new terminal window. With one method, it starts at once. When the program exits successfully, the city opens the session again and resends the task. The login is the agent's own program, configured by the agent itself; the city does not read the tokens it writes, does not enrol them into the Vault, and does not turn subscription access into a native provider API key. The city never starts a claude.ai subscription login: that method is removed from the list Claude Code's ACP adapter offers, and its Console login may be started. <!-- v0.0.11-verify -->

**Versions.** An agent from the catalog is pinned to the version in the release's snapshot, so adding it does not depend on the network. **Refresh** on the ACP page asks the registry's index once, conditionally, and marks an added agent whose newer version exists; moving to it is one activation and a fresh consent, because the command changed. A pasted or detected agent keeps the version its command names. This release does not download, verify or unpack registry agents distributed as binary archives: such an agent can be added once it is installed and found on this machine, or by pasting its command. <!-- v0.0.11-verify -->

The city opens ACP version 1 over stdio, initializes the client with its name and version, declares that it can run terminal login and open a URL the agent asks the User to visit, opens a session in the run's working tree, and sends a text prompt. It advertises no delegated filesystem or terminal capability and sends an empty `mcpServers` list. The agent runs its own tools and selects its own model; city MCP configuration and `dispatch --model` do not configure it. Each run records the agent's id, the version the agent reported, and the digest of the launch specification you consented to, never the command line or its paths. See [ACP tool calls](https://agentclientprotocol.com/protocol/tool-calls) for the upstream reporting contract and the city's [session implementation](../crates/agent_protocols/src/harness/session.rs) for the capabilities it actually offers. <!-- v0.0.11-verify -->

Agent updates enter the Ledger as reports of what the agent said. They are not proof that every tool operation passed the city's native tool policy. An external process can use its own network and tools; a city network setting is not a network sandbox around that process. It starts without the city's `SPRAWLING_SECRET_*` variables and `SPRAWLING_PAIRING_TOKEN`, but it runs as your account, so it can read the native key file and drive the city as you can ([SECURITY.md](../SECURITY.md)). The city refuses an agent dispatch into a `confidential` building because the agent can send the room to its vendor. To keep the building confidential, remove the agent choice and use an appropriate native resident.

| Failure | Next action |
|---|---|
| Unknown agent id, `E_CONFIG_INVALID` | Add the agent, or use the id of an `[[agent]]` row or a built-in entry, and edit the configuration layer named by the refusal |
| Pasted text refused | Paste one program and its arguments, or one `agent_servers` block or `agent.json`; shell syntax is not interpreted |
| Consent refused because the specification changed | Read the new card and consent again |
| Missing runner, missing executable or failed child launch | Make the program visible at the path recorded at consent, or add the agent again |
| Authentication required, `E_AUTH_REQUIRED` | Choose one of the login methods offered; do not enrol the agent's subscription tokens into the city <!-- v0.0.11-verify --> |
| `--model` with an agent resident, `E_CONFIG_INVALID` | Omit the override, or remove the agent selection to use a native model |
| Confidential building, `E_GATE_DENIED` | Keep confidentiality and use a native resident |
| Incompatible ACP reply or closed child output | Follow the refusal's recovery; if a prompt was already sent, inspect the room's files and history before another dispatch |

## Connect an MCP tool server

A native resident reaches servers from the same city/building/resident configuration ladder. A nearer layer's stated MCP list replaces the farther list; it does not append one server to an inherited list. Preserve the entries you still need when replacing a list. The [configuration parser](../crates/city/src/config_layers/mcp.rs) accepts exactly one `command` or `url` per entry, rejects unknown keys and duplicate labels within a layer, and permits only non-empty ASCII lowercase letters and digits in a label.

For stdio, install the server first and make its executable visible to the serving process. `command` is an executable, with separate `args`; it is not a shell command string. The city starts it and speaks JSON-RPC on its pipes, so ordinary logging must not corrupt stdout. The server starts without the city's `SPRAWLING_SECRET_*` variables and `SPRAWLING_PAIRING_TOKEN`, and on Windows without the city's console window; a credential it needs reaches it only through its own `env` entry. <!-- v0.0.11-verify --> For a URL, start or obtain the service separately: the city connects to that URL and does not launch it. These match the [MCP transport specification](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports).

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

This read opens connections, initializes MCP, sends the initialized notification and lists tools; it can start a stdio process and contact a service even though it writes no Ledger event. It is an explicit probe, not a background timer. Unlike a confidential run, this probe does not check the building's confidentiality before connecting; request it only when you intend to reach those servers. An empty server list can also mean the configuration could not be read. Run `sprawling check <city-directory>` to diagnose configuration errors rather than treating an empty health answer as validation. The reply includes transport, target, negotiated revision, remote tool name, city tool name and input schema. The client revision requested has one definition in [handshake](../crates/agent_protocols/src/mcp/handshake.rs); use the reply to see what the server negotiated. [The live probe](../crates/accounting/src/views/mcp_health.rs) defines these diagnostic outcomes.

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

Give the existing agent access to the installed executable and a working directory where it can read [the wire reference](wire.md). Native provider credentials remain the city's responsibility; the driving agent needs neither those keys nor an MCP plugin to send a task. On the machine that serves the city, `call` and `dispatch` read the key file the city writes for its port, readable only by your account, so the driving agent passes no token; it must run as that account. For access from outside loopback, obtain the city's pairing token from Settings and supply `--token` to `call` or `dispatch`; a provider key or vendor login token is not a pairing token. <!-- v0.0.11-verify --> Keep it out of shared transcripts and command recordings.

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

Detached success prints the run identifier when the run starts; it does not certify completed work. Attached output is the received JSONL frames. The convenience verb supplies an empty `goal`; use the ordinary wire Dispatch described in `docs/wire.md` when you need a separate goal or other fields. A native resident may take `--model`; an agent resident rejects that override.

`call` also accepts `--until <kind>` to wait for a serialized EventKind and `--json` to print a refusal in machine-readable form. `--quiet-ms` bounds silence, not total run duration. A larger value changes waiting, not whether a command was accepted. Do not put an automatic retry around a dispatch: a new invocation creates a new idempotency key and may start the work twice.

| Exit | What was observed | Next action |
|---|---|---|
| 0 | An answer or the requested milestone | Read the answer; dispatch success alone does not prove its task's acceptance criteria |
| 1 | A refusal or a broken exchange | Read the code and recovery; a broken exchange after submission may leave an unknown effect |
| 2 | Unreadable arguments or frame | Fix the grammar before sending again |
| 3 | Silence or an unfinished wait | Inspect history and run state; it may still be working |
| 4 | No city completed the greeting | Check the served address and release/schema compatibility |

Stopping the CLI's wait or closing its terminal sends no cancellation command to the city. To stop the work, use the city's stop control for that run, or send the wire `Cancel` command through `call -` using the observed run identifier and an idempotency key in the wire reference's format. The [command type](../crates/wire/src/command/kind.rs) owns those fields; cancellation is not a `dispatch` flag. Observe that run's `run_frozen` event and completion before deciding what to do next. Native runs hear cancellation at safe points; an agent turn sends `session/cancel` when it observes the stop. Cancellation cannot undo external effects already performed, and a lost answer still needs inspection before retrying.

The [wire client specification](../crates/sprawling/spec/WireClient.lean) and [CLI grammar](../crates/sprawling/src/main/verbs.rs) define these outcomes and flags. Do not infer a safe retry from a timeout or a nonzero exit alone.

### A prompt for an existing agent

Replace the three angle-bracket values with your installed executable, directory and served address before giving this prompt to an agent:

> Use `<executable>` from working directory `<directory>` to drive the served sprawling city at `<host:port>`. Read `docs/wire.md` from a repository checkout, or the matching release's wire reference, and run `help call` and `help dispatch` on that executable before sending work. Start with a `city_view` query and use an existing room chosen by me. Send one task through `dispatch`, or one UTF-8 JSON frame through `call -` when a separate goal is needed. Read exit codes and refusals; after silence or a broken exchange, inspect run state and history before retrying. Request pairing only if the city requires it, keep credentials out of files and transcripts, and report the run identifier and observed result.

### The incoming `/acp` request endpoint

The city's HTTP `POST /acp` route is an incoming admission path for an editor or another client. It is distinct from the outbound stdio ACP session above; pointing an editor's standard ACP stdio configuration at this HTTP URL does not make them interchangeable. This guide supplies no editor plugin that implements that conversion.

Send a JSON body with non-empty `addr`, `task` and `goal`, plus a credential: from this machine, the native key in the key file the city writes for its port; from elsewhere, the pairing `token` obtained from Settings. The route requires a credential from every caller, loopback included, refuses a browser whose `Origin` is not the listener's own, <!-- v0.0.11-verify --> refuses reserved city addresses, and passes an admitted request to the ordinary dispatch worker. Keep the token in a private request source rather than a committed fixture. The parser's grammar is [Incoming::parse](../crates/agent_protocols/src/acp.rs).

HTTP 202 returns `run`, `turns` and `finished`; the initial response has zero turns and is unfinished. Its `run` value is the admission's derived idempotency identifier, not evidence that the worker has created a RunId or finished the task. A refusal returns HTTP 403 with its reason and recovery. The response has no conversation stream and cannot report a later worker refusal after the request has been queued. Observe the city through its ordinary wire and history for the actual run outcome.
