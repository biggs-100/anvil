# Forge

**Create, run, inspect, and share reproducible development environments — for humans, tools, and AI agents.**

## How to Use

This file configures how AI agents work on this project. The orchestrator agent reads it at session start.

### Key Rules

- All substantial changes MUST follow SDD (Spec-Driven Development): proposal → specs → design → tasks → apply → verify → archive.
- Delegate ALL complex work to sub-agents. Do not implement, explore, or write specs inline for tasks involving 2+ files or 4+ file reads.
- The core is frozen at 1.0. Do NOT modify `crates/forge-core/src/api/v1.rs` or frozen types in `crates/forge-core/src/types.rs`.
- Generated technical artifacts default to English regardless of persona.

## Skills

| Skill | Trigger | Path |
|-------|---------|------|
| `branch-pr` | Create Gentle AI pull requests with issue-first checks. Trigger: creating, opening, or preparing PRs for review. | `~/.config/opencode/skills/branch-pr/SKILL.md` |
| `chained-pr` | Trigger: PRs over 400 lines, stacked PRs, review slices. Split oversized changes into chained PRs that protect review focus. | `~/.config/opencode/skills/chained-pr/SKILL.md` |
| `cognitive-doc-design` | Design docs that reduce cognitive load. Trigger: writing guides, READMEs, RFCs, onboarding, architecture, or review-facing docs. | `~/.config/opencode/skills/cognitive-doc-design/SKILL.md` |
| `comment-writer` | Write warm, direct collaboration comments. Trigger: PR feedback, issue replies, reviews, Slack messages, or GitHub comments. | `~/.config/opencode/skills/comment-writer/SKILL.md` |
| `customize-opencode` | Use ONLY when the user is editing or creating opencode's own configuration. | `~/.config/opencode/skills/customize-opencode/SKILL.md` |
| `go-testing` | Trigger: Go tests, go test coverage, Bubbletea teatest, golden files. Apply focused Go testing patterns. | `~/.config/opencode/skills/go-testing/SKILL.md` |
| `issue-creation` | Create Gentle AI issues with issue-first checks. Trigger: creating GitHub issues, bug reports, or feature requests. | `~/.config/opencode/skills/issue-creation/SKILL.md` |
| `judgment-day` | Trigger: judgment day, dual review, adversarial review, juzgar. Run blind dual review, fix confirmed issues, then re-judge. | `~/.config/opencode/skills/judgment-day/SKILL.md` |
| `professional-web-design` | Trigger: rediseñar, redesign, professional design, anti-ai design, pro design, diseño profesional. | `~/.config/opencode/skills/professional-web-design/SKILL.md` |
| `skill-creator` | Trigger: new skills, agent instructions, documenting AI usage patterns. Create LLM-first skills with valid frontmatter. | `~/.config/opencode/skills/skill-creator/SKILL.md` |
| `skill-improver` | Trigger: improve skills, audit skills, refactor skills, skill quality. Audit and upgrade existing LLM-first skills. | `~/.config/opencode/skills/skill-improver/SKILL.md` |
| `skill-registry` | Trigger: update skills, skill registry, actualizar skills, after skill changes. Index available skills by trigger and path. | `~/.config/opencode/skills/skill-registry/SKILL.md` |
| `work-unit-commits` | Plan commits as reviewable work units. Trigger: implementation, commit splitting, chained PRs, or keeping tests and docs with code. | `~/.config/opencode/skills/work-unit-commits/SKILL.md` |

## SDD Workflow Configuration

### Model Assignments by Phase

| Phase | Model | Cost Level |
|-------|-------|------------|
| sdd-explore | cheap (flash/mini) | 🟢 |
| sdd-propose | cheap | 🟢 |
| sdd-spec | cheap | 🟢 |
| sdd-design | **capable** | 🔴 |
| sdd-tasks | cheap | 🟢 |
| sdd-apply | **capable** | 🔴 |
| sdd-verify | cheap | 🟢 |
| sdd-archive | cheap | 🟢 |

### Mandatory Workflow Rules

1. **STRICT sub-agent delegation** — every task, investigation, or code change MUST be delegated to a sub-agent. Do NOT do work inline.
2. **4R review before EVERY commit** — before any commit, push, or PR, run all four reviews in parallel: `review-risk`, `review-resilience`, `review-readability`, `review-reliability`.
3. **Judgment Day after design and apply phases** — run `judgment-day` after `sdd-design` and `sdd-apply`.
4. **Cheap models for low-risk phases** — explore, propose, spec, tasks, verify, archive use cheap models. Only design and apply use capable models.
5. **Fresh-context reviewers** — all review phases (4R, judgment-day) MUST use fresh independent context.
