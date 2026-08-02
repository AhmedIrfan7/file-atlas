# Security Policy

File Atlas touches user files and, in future versions, will support optional integrations with local AI models and cloud services. Because a bug in this class of software can cost real people real data, we take security seriously.

## Supported versions

Until File Atlas ships v1.0, only the latest commit on `main` is supported. From v1.0 onward we will support the current minor release and the one prior.

| Version | Supported |
| ------- | --------- |
| `main`  | Yes       |
| < 1.0   | No        |

## Reporting a vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Instead, report privately to `ahmedirfancodes@gmail.com` with the subject line `[SECURITY] File Atlas: <short description>`. Include:

- A description of the issue and its impact
- Steps to reproduce
- Affected version or commit
- Any proof-of-concept code or files (please do not include anyone else's personal data)

You should receive an acknowledgement within 72 hours. A more detailed response will follow within 7 days with our assessment and an expected fix timeline. We will keep you updated as we work on the fix, and we will credit you in the release notes unless you prefer to remain anonymous.

## Scope

In scope:

- The desktop application itself (Rust core, IPC boundary, frontend)
- Anything the app installs on the user's system (autoupdater, shell integrations)
- Any first-party build artifact distributed from this repository
- Any first-party website or documentation site under our control

Out of scope:

- Third-party integrations we do not ship (for example, a user's own cloud provider)
- Findings that require the attacker to already have full local administrator access
- Denial of service against a single local process the user started themselves

## Safe harbor

We support responsible security research. If you make a good-faith effort to comply with this policy, we will not pursue legal action against you.

## Preferred fix path

If the vulnerability is low severity and already public elsewhere, a Pull Request with a fix is welcome. For anything higher severity, please report privately first and we will coordinate the fix, tests, disclosure timing, and credit.
