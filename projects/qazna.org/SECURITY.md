# Security reporting

The current development branch is intended for local experiments. Public multi-user operation is not implemented. The operator token grants access to all operator routes; it does not replace authorization for individual residents or accounts.

Do not publish active credentials, personal data or exploit details in public issues. Use GitHub's **Report a vulnerability** feature if it is enabled for this repository; otherwise arrange a private channel with the maintainer through their public profile first. Private reporting and a round-the-clock response service are not assumed to exist.

Include the commit, environment, a minimal synthetic-data reproduction and the impact on accounting integrity, privacy or availability. There is no established remediation deadline yet. An external pilot requires access controls, protected transport, dependency review and verified backup restoration.

See the [AI threat model](docs/ai-safety.md) for implemented evidence checks, constrained model tests, worker fencing and explicit deployment gaps. AI commentary does not authorize economic actions.
