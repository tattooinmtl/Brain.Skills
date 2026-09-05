# Claw Code Ecosystem & Philosophy

## The Three-Part System

### 1. OmX (oh-my-codex)
**Workflow Layer**

Transforms short directives into structured execution:
- Planning keywords
- Execution modes
- Persistent verification loops
- Parallel multi-agent workflows

GitHub: https://github.com/Yeachan-Heo/oh-my-codex

### 2. clawhip
**Event & Notification Router**

Watches:
- Git commits
- tmux sessions
- GitHub issues and PRs
- Agent lifecycle events
- Channel delivery

Keeps monitoring and delivery OUTSIDE the coding agent's context window so agents can stay focused on implementation.

GitHub: https://github.com/Yeachan-Heo/clawhip

### 3. OmO (oh-my-openagent)
**Multi-Agent Coordination**

Handles:
- Planning
- Handoffs
- Disagreement resolution
- Verification loops across agents

When Architect, Executor, and Reviewer disagree, OmO provides structure for convergence.

GitHub: https://github.com/code-yeongyu/oh-my-openagent

## Core Philosophy

### The Human Interface Is Discord

The real human interface is a Discord channel, not tmux, Vim, SSH, or terminal multiplexer.

A person can:
- Type a sentence from a phone
- Walk away
- Sleep
- Do something else

The claws read the directive, break it into tasks, assign roles, write code, run tests, argue over failures, recover, and push when the work passes.

### The New Bottleneck

The bottleneck is no longer typing speed.

When agent systems can rebuild a codebase in hours, the scarce resource becomes:
- Architectural clarity
- Task decomposition
- Judgment
- Taste
- Conviction about what is worth building
- Knowing which parts can be parallelized

### What Still Matters

As coding intelligence gets cheaper:
- Product taste
- Direction
- System design
- Human trust
- Operational stability
- Judgment about what to build next

**The job of the human is NOT to out-type the machine.**
**The job of the human is to decide what deserves to exist.**

## What Claw Code Demonstrates

A repository can be:
- Autonomously built in public
- Coordinated by claws/lobsters rather than human pair-programming
- Operated through a chat interface
- Continuously improved by structured planning/execution/review loops
- Maintained as a showcase of the coordination layer

## Related Ecosystem Projects

| Project | Description |
|---------|-------------|
| clawhip | Event and notification router |
| oh-my-openagent | Multi-agent workflow coordination |
| oh-my-claudecode | Claude Code enhancements |
| oh-my-codex | Codex workflow automation |

## Community

- **Discord**: UltraWorkers Community
- **GitHub**: ultraworkers/claw-code
- **Stars**: 173k+
- **Forks**: 105k+

## Key Principle

**Humans set direction; claws perform the labor.**

The code is evidence. The coordination system is the product lesson.
