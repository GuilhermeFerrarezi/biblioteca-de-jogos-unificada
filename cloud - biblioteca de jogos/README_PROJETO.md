# Agent Pack - cloud - biblioteca de jogos

This folder stores the project-specific agent and skill base for the Unified Game Library project.

## Purpose

- Keep scope, architecture, security, implementation, and QA separated.
- Serve as the operational base for planning, review, and execution.
- Centralize the project context in `project-profile.md`.

## Current project

- Fill out `project-profile.md` before reusing the agents in a new session.
- Use `project-profile.template.md` as the starting point for new contexts.
- Check the examples in `../cloud/examples` when you want to compare briefing formats.

## Suggested order

1. `00-project-manager.md`
2. `01-platform-research-agent.md`
3. `02-architecture-agent.md`
4. `03-security-auth-agent.md`
5. `04-backend-provider-agent.md`
6. `05-frontend-ux-agent.md`
7. `06-metadata-agent.md`
8. `11-senior-database-agent.md`
9. `08-senior-backend-development-agent.md`
10. `09-senior-frontend-development-agent.md`
11. `07-qa-compliance-agent.md`
12. `10-senior-integration-qa-agent.md`

## How to adapt

- Replace concrete examples with concepts from the current project.
- Keep the files short and decision-oriented.
- Update the agents when the project domain changes.
- Use one main agent and, when it makes sense, supporting agents for research, implementation, and review.
