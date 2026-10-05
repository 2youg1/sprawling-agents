# Security policy

## Reporting

Report vulnerabilities privately through this repository's **Security → Report a vulnerability** page. Include the version or commit, platform, configuration, reproduction steps, impact and the boundary crossed. Use redacted examples and omit live credentials. Keep security-sensitive reports out of public issues, pull requests and discussions; use those public channels for ordinary bugs.

There is no bug bounty. Public credit is optional: remain unnamed or choose a name or handle. We publish credit only with your consent.

## Trust and scope

sprawling runs under the local User's account. Installed harnesses and MCP tool servers execute their own code and are trusted components; a room or git worktree alone does not contain their access to the host. Built-in tools enforce the city's policy, and host-command confinement depends on the selected mechanism, platform and available prerequisites. A copied working tree changes where a command starts; it does not prevent access to other paths the account can reach. See [execution boundaries](docs/operating.md#how-exec-is-confined) for the guarantees of each supported configuration.

Configuration and records carry credential references; the vault holds credential plaintext. Network pairing and local-only permissions restrict access to the city's interfaces. Neither loopback binding nor secret scanning protects against every malicious process already running as the same account, and scanning cannot detect every credential format.

Report unauthorized network access, breaches of enforced file or credential boundaries, and behavior exceeding a supported configuration's actual isolation. Prompt injection or a trusted tool carrying out an explicitly permitted action alone does not establish a bypass; identify the unauthorized effect. Prior local write access is not a new vulnerability unless sprawling grants that access or crosses a further protected boundary.

## Supported versions

sprawling is in <!-- xtask:begin maturity:word -->alpha<!-- xtask:end -->. Confirmed fixes ship with the next release; we aim to publish fixes promptly. Older releases do not receive backports.
