# Security policy

## Reporting

Report vulnerabilities privately through [GitHub's Report a vulnerability page](https://github.com/2youg1/sprawling-agents/security/advisories/new). Include the version or commit, platform, configuration, reproduction steps, impact and the boundary crossed. For execution reports, include the tool's confinement disclosure or `sprawling doctor` output. Use redacted examples and omit live credentials. Keep security-sensitive reports out of public issues, pull requests and discussions; use [public issues](https://github.com/2youg1/sprawling-agents/issues) for ordinary bugs.

There is no bug bounty. Public credit is optional: remain unnamed or choose a name or handle. We publish credit only with your consent.

## Trust and scope

Report unauthorized access across an enforced network, filesystem, credential or execution boundary. sprawling runs with the local User's rights, and user-approved external programs keep their own host access. For prompt injection, trusted tools and prior local access, describe the additional unauthorized effect or boundary crossing. The [threat model](threat-model.md) records what each supported interface holds and where its guarantees end.

## Supported versions

The first component of a sprawling version expresses maturity. Confirmed fixes ship with the next release; we aim to publish fixes promptly. Older releases do not receive backports.
