# Duo Mode

Duo mode implements the **player-coach autocoding** paradigm (based on the g3 paper) for iterative development with built-in validation.

## Overview

Two roles collaborate in a loop:

- **Player**: implements requirements (builder role)
- **Coach**: validates implementation against requirements (critic role)

Workflow: `init → player → coach → advance → (repeat until approved)`

## Tools

| Tool | Description |
|------|-------------|
| `duo_init` | Initialize a new Duo session with requirements |
| `duo_player` | Execute player phase (code generation) |
| `duo_coach` | Execute coach phase (validation and feedback) |
| `duo_advance` | Advance to the next turn or approve |
| `duo_status` | Show current session state |

## CLI Commands

```bash
minimax duo start --requirements docs/requirements.md   # Start a new workflow
minimax duo start --requirements spec.md --workspace .   # Start in current directory
minimax duo continue <session-id>                        # Resume a saved session
minimax duo sessions                                     # List all saved sessions
```

## Session Persistence

Sessions are automatically saved after each turn to `~/.axiom/sessions/duo/` as JSON files. Each session records:

- Requirements and configuration
- Phase history (player/coach turns)
- Quality scores and coach feedback
- Current state for resume capability

Functions: `save_session()`, `load_session()`, `list_sessions()`, `delete_session()`.

## TUI Interface

### DuoView Modal

The `DuoView` component (`src/tui/views/duo_view.rs`) provides a dedicated modal overlay:

- **Phase indicator** with color coding (Player = green, Coach = blue)
- **Turn counter** and progress bar
- **Quality scores** visualization
- **Feedback history** display
- **Loop visualization** showing Player ↔ Coach progression

### DuoSessionPicker

The `DuoSessionPicker` (`src/tui/duo_session_picker.rs`) is a session browser:

- Fuzzy search over saved sessions
- Session metadata display (phase, turns, quality score)
- Resume capability from the picker

### Footer Progress Indicator

When Duo mode is active, the TUI footer shows a real-time progress indicator:

- `🎮 Player Phase (Turn 2/10)` during player execution
- `🏆 Coach Phase (Turn 2/10)` during coach validation

## Approval Behavior

Duo mode auto-approves tools during the player-coach loop. File operations are sandboxed to the workspace directory. The coach reads implementation files for validation; the player writes generated code.

## Configuration

Duo mode uses `DuoConfig` from the main configuration system. Relevant settings:

- Maximum turns per session
- Quality score thresholds
- Model selection (defaults to `MiniMax-M2.1-Coding`)

## Related

- `MODES.md` - Overview of all TUI modes and approval behavior
- `ARCHITECTURE.md` - Code layout and module organization
- `../PLAN.md` - Implementation plan and status
