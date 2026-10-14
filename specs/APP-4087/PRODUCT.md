# APP-4087: Fix AI Terminal skill and MCP home config paths after watcher unification Product Spec
## Summary
The immediate regression is that `oz agent run --skill <name>` no longer finds AI Terminal skills stored in AI Terminal’s home-relative config directory on Linux and Windows. APP-3945 intentionally centralized AI Terminal-owned filesystem watching to avoid recursively watching `.ai-terminal*/worktrees`, but it also tied AI Terminal skill lookup to platform app data paths. That works on macOS because AI Terminal’s app data path is home-relative and channel-aware, but it breaks non-macOS users because app data follows XDG/AppData conventions instead of AI Terminal’s `.ai-terminal*` home config directory convention.
Fixing the skill regression led us to audit other AI Terminal-owned home config paths affected by the same watcher unification. MCP has the same shape: AI Terminal’s file-based MCP config should live next to other home-relative AI Terminal config for the current app environment. APP-4087 should restore those environment-aware home paths without undoing APP-3945’s worktree-watch safety.
## Problem
Before APP-3945, AI Terminal skill discovery could find skills in AI Terminal’s home config directory, such as `~/.ai-terminal/skills`. After APP-3945, `SkillProvider::AiTerminal` stopped going through generic home-provider watching and instead relied on the centralized AI Terminal watcher and app data paths. That prevented broad recursive watches under `~/.ai-terminal`, which was intended, but it also meant that home-relative AI Terminal skill directories stopped being considered when app data was somewhere else.
Linux and Windows expose the bug because their app data directories differ from AI Terminal’s home-relative `.ai-terminal*` config directories. A Stable user can therefore have a valid skill at `~/.ai-terminal/skills/foo/SKILL.md`, run `oz agent run --skill foo`, and see the skill fail to resolve because the resolver and watcher are looking in the platform app data directory.
macOS hides most of this because Stable and Preview commonly resolve app data to `~/.ai-terminal`, while Dev and Local resolve to environment-specific home directories such as `~/.ai-terminal-dev` and `~/.ai-terminal-local`. The desired behavior is not simply “always use `~/.ai-terminal`”; it is “use the home-relative AI Terminal config directory for the current AI Terminal app environment.”
While investigating Skills, we checked MCP because its global config has the same shape. AI Terminal MCP config should use the same environment-aware home config directory as Skills, e.g. `~/.ai-terminal/.mcp.json`, `~/.ai-terminal-dev/.mcp.json`, or `~/.ai-terminal-local/.mcp.json` depending on channel/profile.
## Goals
- Preserve the APP-3945 invariant that AI Terminal does not recursively watch `.ai-terminal*/worktrees`.
- Restore `oz --skill <name>` resolution for AI Terminal home skills on Linux, Windows, and macOS.
- Preserve environment isolation for Dev, Local, Integration, OpenAiTerminal, and development profiles.
- Use a single purpose-specific home config path helper for AI Terminal Skills and MCP.
- Keep `data_dir()` and `config_local_dir()` for their existing app-managed configuration responsibilities.
- Keep AI Terminal-specific filesystem watching centralized instead of reintroducing ad hoc recursive watchers in `SkillWatcher` or `FileMCPWatcher`.
## Non-goals
- Changing the public skill file format or MCP config schema.
- Changing project-level skill or MCP discovery.
- Migrating existing files between platform app data directories and home-relative `.ai-terminal*` directories.
- Treating non-macOS XDG/AppData `data_dir()/skills` or `data_dir()/.mcp.json` as AI Terminal Skills or MCP sources of truth.
- Changing non-AI Terminal provider paths such as `~/.agents/skills`, `~/.claude/skills`, `~/.codex/config.toml`, or project provider paths.
- Introducing a generic filtering API in `repo_metadata::DirectoryWatcher`.
## Figma / design references
Figma: none provided.
## User Experience
### AI Terminal home skills
- A Stable user can store a skill at `~/.ai-terminal/skills/<skill-name>/SKILL.md`.
- Dev, Local, Integration, OpenAiTerminal, and profiled builds use their own home-relative AI Terminal config directories, such as `~/.ai-terminal-dev/skills`, `~/.ai-terminal-local/skills`, or `~/.ai-terminal-local-<profile>/skills`.
- Running `oz agent run --skill <skill-name> ...` resolves the skill from the current app environment’s AI Terminal home skills directory even when platform app data is elsewhere.
- AI Terminal home skill resolution continues to take precedence over project skill resolution for unqualified skill names.
- The resolver must not require the asynchronous `SkillManager` cache or filesystem watcher to be warmed before `oz --skill` works.
### AI Terminal home MCP config
- A user can configure file-based MCP servers for the current AI Terminal app environment at `<ai-terminal-home-config-dir>/.mcp.json`.
- Examples include `~/.ai-terminal/.mcp.json`, `~/.ai-terminal-dev/.mcp.json`, and `~/.ai-terminal-local/.mcp.json`.
- When the MCP file is created, edited, moved into place, or deleted, AI Terminal updates detected file-based MCP servers without requiring a restart, as long as the relevant parent path is watchable.
- AI Terminal MCP config is scoped as a user-level config, not as a project config or platform app-data config.
### Worktree exclusion
- Activity under `.ai-terminal*/worktrees` must not trigger reloads for themes, workflows, tab configs, settings, skills, or MCP config.
- Supporting AI Terminal home skills must not be implemented by recursively watching all possible `.ai-terminal*` directories.
- Supporting AI Terminal home MCP config must not be implemented by recursively watching all possible `.ai-terminal*` directories.
### Existing app paths
- Channel-aware app files under `data_dir()` continue to work as before for non-Skills/MCP app config.
- `data_dir()` remains the root for channel-scoped themes, workflows, launch configs, tab configs, and other app-managed files.
- `config_local_dir()` remains the root for platform-specific config files such as `settings.toml`, `keybindings.yaml`, and `user_preferences.json`.
## Success Criteria
- `oz agent run --skill <name>` can resolve `<ai-terminal-home-config-dir>/skills/<name>/SKILL.md` from a cold start.
- Skill resolution still finds project skills when no matching AI Terminal home skill exists.
- AI Terminal home skills still take precedence over project skills for unqualified skill names.
- File-based MCP detection includes `<ai-terminal-home-config-dir>/.mcp.json` as a user-scoped AI Terminal provider config when present.
- Dev/Local/Profiled builds use isolated `.ai-terminal*` home config directories instead of Stable’s `~/.ai-terminal` directory.
- No code path treats non-macOS XDG/AppData `data_dir()/skills` or `data_dir()/.mcp.json` as AI Terminal home Skills or MCP sources.
- No code path reintroduces a generic recursive watcher rooted at `~/.ai-terminal`.
- `.ai-terminal*/worktrees` changes remain excluded from AI Terminal-managed reload behavior.
## Validation
- Add or update unit coverage for `oz --skill` resolving a skill from an explicit AI Terminal home skills directory.
- Add or update unit coverage for AI Terminal home config path helper behavior.
- Add or update unit coverage for MCP path classification so `<ai-terminal-home-config-dir>/.mcp.json` is recognized as an AI Terminal MCP config path.
- Add or update watcher helper tests to verify managed Skills/MCP helpers return the current environment’s AI Terminal home paths.
- Add or update skill utility tests so only the current environment’s AI Terminal home skills directory is classified as home AI Terminal skills.
- Run targeted Rust tests for path helpers, skill resolution, skill file watcher utilities, MCP provider/path helpers, and AI Terminal managed path filtering.
## Alternatives Considered
- Use only hardcoded `~/.ai-terminal` for Skills/MCP. Rejected because it loses Dev/Local/Profile environment isolation.
- Keep using `data_dir()` for Skills/MCP. Rejected because non-macOS app data paths are XDG/AppData paths, not AI Terminal’s home-relative `.ai-terminal*` config paths.
- Use `config_local_dir()` for Skills/MCP. Rejected because non-macOS config-local paths are also platform project directories, not home-relative `.ai-terminal*` paths.
- Add only a resolver fallback for `oz --skill`. Rejected because it fixes cold CLI resolution but leaves app hot reload, skill discovery, and MCP path behavior inconsistent.
- Re-add AI Terminal to generic home-provider watchers. Rejected because that watcher shape can recursively watch `.ai-terminal*` parents and reintroduce `.ai-terminal*/worktrees` churn.
- Watch all of `~/.ai-terminal` recursively and filter in consumers. Rejected because it recreates the broad watcher shape APP-3945 was designed to remove.
## Open Questions
- Should AI Terminal proactively create the current environment’s AI Terminal home config directory on startup, or only watch it when it already exists? The implementation should prefer the least invasive approach unless product explicitly wants fresh installs to create these paths.
