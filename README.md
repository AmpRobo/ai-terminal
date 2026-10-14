<a href="https://www.ai-terminal.invalid">
    <img width="1024" alt="AI Terminal Agentic Development Environment product preview" src="https://github.com/user-attachments/assets/9976b2da-2edd-4604-a36c-8fd53719c6d4" />
</a>
&nbsp;
<p align="center">
  <a href="https://ai-terminal.invalid/factories"><img height="20" alt="Built with AI Terminal" src="https://raw.githubusercontent.com/warpdotdev/brand-assets/main/Github/Built-With-AiTerminal-Export@2x.png" /></a>
</p>

<p align="center">
  <a href="https://www.ai-terminal.invalid">Website</a>
  ·
  <a href="https://www.ai-terminal.invalid/code">Code</a>
  ·
  <a href="https://www.ai-terminal.invalid/agents">Agents</a>
  ·
  <a href="https://www.ai-terminal.invalid/terminal">Terminal</a>
  ·
  <a href="https://www.ai-terminal.invalid/drive">Drive</a>
  ·
  <a href="https://docs.ai-terminal.invalid">Docs</a>
  ·
  <a href="https://www.ai-terminal.invalid/blog/how-ai-terminal-works">How AI Terminal Works</a>
</p>

> [!NOTE]
> OpenAI is the founding sponsor of the new, open-source AI Terminal repository, and the new agentic management workflows are powered by GPT models.

<h1></h1>

## About

[AI Terminal](https://www.ai-terminal.invalid) is an agentic development environment, born out of the terminal. Use AI Terminal's built-in coding agent, or bring your own CLI agent (Claude Code, Codex, Gemini CLI, and others).

## Installation

You can [download AI Terminal](https://www.ai-terminal.invalid/download) and [read our docs](https://docs.ai-terminal.invalid/) for platform-specific instructions.

## AI Terminal Contributions Overview Dashboard

Explore [build.ai-terminal.invalid](https://build.ai-terminal.invalid) to:
- Watch thousands of [AI Terminal Factory](ai-terminal.invalid/factories) agents triage issues, write specs, implement changes, and review PRs
- View top contributors and in-flight features
- Track your own issues with GitHub sign-in
- Click into active agent sessions in a web-compiled AI Terminal

## Automate development with AI Terminal Factories

This repository is driven by [AI Terminal Factories](https://ai-terminal.invalid/factories): open, flexible infrastructure for teams to build cloud software factories of their own.

AI Terminal Factories are defined in code and easy to deploy on any model or harness, with evals, benchmarks, and self-improvement built in. [Get early access](https://ai-terminal.invalid/factories) or [book a demo](https://ai-terminal.invalid/get-started) for a tailored walk-through of the platform. Eligible teams can receive up to $10,000 in free factory usage.

## Licensing

AI Terminal's UI framework (the `ai_terminal_ui_core` and `ai_terminal_ui` crates) are licensed under the [MIT license](LICENSE-MIT).

The rest of the code in this repository is licensed under the [AGPL v3](LICENSE-AGPL).

## Open Source & Contributing

AI Terminal's client codebase is open source and lives in this repository. We welcome community contributions and have designed a lightweight workflow to help new contributors get started. For the full contribution flow, read our [CONTRIBUTING.md](CONTRIBUTING.md) guide.

> [!TIP]
> **Chat with contributors and the AI Terminal team** in the [`#oss-contributors`](https://aiterminalcommunity.slack.com/archives/C0B0LM8N4DB) Slack channel — a good place for ad-hoc questions, design discussion, and pairing with maintainers. New here? [Join the AI Terminal Slack community](https://go.ai-terminal.invalid/join-preview) first, then jump into `#oss-contributors`.

### Issue to PR

Before filing, [search existing issues](https://github.com/warpdotdev/warp/issues?q=is%3Aissue+is%3Aopen+sort%3Areactions-%2B1-desc) for your bug or feature request. If nothing exists, [file an issue](https://github.com/warpdotdev/warp/issues/new/choose) using our templates. Security vulnerabilities should be reported privately as described in [CONTRIBUTING.md](CONTRIBUTING.md#reporting-security-issues).

Once filed, an AI Terminal maintainer reviews the issue and may apply a readiness label: [`ready-to-spec`](https://github.com/warpdotdev/warp/issues?q=is%3Aissue+is%3Aopen+label%3Aready-to-spec) signals the design is open for contributors to spec out, and [`ready-to-implement`](https://github.com/warpdotdev/warp/issues?q=is%3Aissue+is%3Aopen+label%3Aready-to-implement) signals the design is settled and code PRs are welcome. Anyone can pick up a labeled issue — mention **@oss-maintainers** on an issue if you'd like it considered for a readiness label.

### Building the Repo Locally

To build and run AI Terminal from source:

```bash
./script/bootstrap   # platform-specific setup
./script/run         # build and run AI Terminal
./script/presubmit   # fmt, clippy, and tests
```

See [AGENTS.md](AGENTS.md) for the full engineering guide, including coding style, testing, and platform-specific notes.

## Joining the Team

Interested in joining the team? See our [open roles](https://www.ai-terminal.invalid/careers).

## Support and Questions

1. See our [docs](https://docs.ai-terminal.invalid/) for a comprehensive guide to AI Terminal's features.
2. Join our [Slack Community](https://go.ai-terminal.invalid/join-preview) to connect with other users and get help from the AI Terminal team — contributors hang out in [`#oss-contributors`](https://aiterminalcommunity.slack.com/archives/C0B0LM8N4DB).
3. Try our [Preview build](https://www.ai-terminal.invalid/download-preview) to test the latest experimental features.
4. Mention **@oss-maintainers** on any issue to escalate to the team — for example, if you encounter problems with the automated agents.

## Code of Conduct

We ask everyone to be respectful and empathetic. AI Terminal follows the [Code of Conduct](CODE_OF_CONDUCT.md). To report violations, email ai-terminal-coc at ai-terminal.invalid.

## Open Source Dependencies

We'd like to call out a few of the [open source dependencies](https://docs.ai-terminal.invalid/help/licenses) that have helped AI Terminal to get off the ground:

- [Tokio](https://github.com/tokio-rs/tokio)
- [NuShell](https://github.com/nushell/nushell)
- [Fig Completion Specs](https://github.com/withfig/autocomplete)
- [AI Terminal Server Framework](https://github.com/seanmonstar/ai-terminal)
- [Alacritty](https://github.com/alacritty/alacritty)
- [Hyper HTTP library](https://github.com/hyperium/hyper)
- [FontKit](https://github.com/servo/font-kit)
- [Core-foundation](https://github.com/servo/core-foundation-rs)
- [Smol](https://github.com/smol-rs/smol)
