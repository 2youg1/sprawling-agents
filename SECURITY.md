# Security policy

## Reporting

Report vulnerabilities privately through [GitHub's Report a vulnerability page](https://github.com/2youg1/sprawling-agents/security/advisories/new). Include the version or commit, platform, configuration, reproduction steps, impact and the boundary crossed. For execution reports, include the tool's confinement disclosure or `sprawling doctor` output. Use redacted examples and omit live credentials. Keep security-sensitive reports out of public issues, pull requests and discussions; use [public issues](https://github.com/2youg1/sprawling-agents/issues) for ordinary bugs.

There is no bug bounty. Public credit is optional: remain unnamed or choose a name or handle. We publish credit only with your consent.

## Trust and scope

sprawling runs under the local User's account. Installed harnesses and MCP tool servers execute their own code and are trusted components; a room or git worktree alone does not contain their access to the host. The kernel's gates and built-in file tools check write domains, reserved paths and read permissions. Those checks govern tool requests, not arbitrary code running under the account. An explicitly permitted `where: host` command runs with that account's access; its working directory does not restrict the paths it can reach.

Host-command confinement depends on the mechanism, platform and available prerequisites. Read the exec tool's disclosure for the run's selected execution route and its limits. `sprawling doctor` reports the machine's detected confinement and prerequisites, which may differ from a run's configured arm. The runtime's [confinement definition](crates/runtime/src/tools/exec/confinement.rs) describes its native and copied-tree arms. A copied working tree alone does not restrict host reads, network access or credentials. Linux namespace confinement also exposes the host filesystem read-only outside the writable copy, so it does not protect the confidentiality of host files. Execution isolation applies to the command or guest, not to the whole sprawling process or external harness.

Configuration and records carry credential references. The Vault uses the platform credential service, with a disclosed session-memory fallback when its startup probe fails; configured environment credentials take precedence. Plaintext is available in process memory and at authorized provider and tool authentication sinks. Credential state shown to clients does not return stored values, but the Vault does not isolate secrets from trusted code or every process running as the same account. Secret scanning cannot detect every credential format.

The city binds to loopback by default and refuses a non-loopback listener without a pairing token. A loopback listener with no token does not distinguish local callers. This listener's token controls access to city interfaces but does not encrypt its HTTP/WebSocket transport or contain a hostile local process.

The separate remote door authenticates paired devices with watch or act permissions, refuses commands classified as local-only, and seals session frames end to end. The route that serves the browser page is trusted to deliver it unchanged: a route that replaces the page can read what the page reads. See [remote access](docs/operating.md#reaching-the-city-from-another-device) for deployment and device permissions.

Report unauthorized network access, breaches of enforced file or credential boundaries, and behavior exceeding a supported configuration's actual isolation. Prompt injection or a trusted tool carrying out an explicitly permitted action alone does not establish a bypass; identify the unauthorized effect. Prior local write access is not a new vulnerability unless sprawling grants that access or crosses a further protected boundary.

## Supported versions

sprawling is in <!-- xtask:begin maturity:word -->alpha<!-- xtask:end -->. Confirmed fixes ship with the next release; we aim to publish fixes promptly. Older releases do not receive backports.
