# Duo Mode Implementation Plan

## Current State

### Implemented ✅
- State machine (`DuoPhase`, `DuoStatus`, `DuoState`)
- 5 Duo tools with full schemas (`duo_init`, `duo_player`, `duo_coach`, `duo_advance`, `duo_status`)
- Configuration system (`DuoConfig`)
- Prompt generation (player/coach)
- 24 unit tests (all passing)
- CLI command structures
- TUI mode integration (`AppMode::Duo`)

### Phase 1 Complete ✅
- **`run_duo_workflow()` connected to LLM API**
  - Function now uses actual `AnthropicClient` for API calls
  - Complete player-coach loop logic implemented
  - Callback-based progress reporting via `EngineEvent::Status`
  - Streaming and non-streaming responses supported

- **File system integration**
  - `read_file()` - read existing code files for validation
  - `write_file()` - save generated code
  - `list_files()` - explore workspace with filtering
  - `validate_path()` - security sandboxing
  - File filtering for common build artifacts

- **Session persistence**
  - `save_session()` - save session to `~/.minimax/sessions/duo/`
  - `load_session()` - load session from disk
  - `list_sessions()` - list all saved sessions
  - `delete_session()` - delete a saved session
  - JSON serialization using serde

- **CLI command handlers**
  - `minimax duo start` - Parse requirements file, start workflow, auto-save
  - `minimax duo continue <id>` - Resume session by ID
  - `minimax duo sessions` - List saved sessions from disk
  - Full error handling and user feedback

- **Progress reporting**
  - Phase transitions reported via engine events
  - Display current phase (Player/Coach) in status
  - Show quality scores and feedback
  - Integration with existing session management

---

## Implementation Plan

### Phase 1: Workflow Execution ✅ COMPLETE

#### Goal
Connect `run_duo_workflow()` to actual LLM API calls using the existing `AnthropicClient` with file system integration.

#### Strategy
- Use existing `AnthropicClient` infrastructure
- Implement file I/O for code generation and validation
- Display progress in TUI via engine events

#### Tasks ✅ Complete

1. **Update `run_duo_workflow()` function** (`src/duo.rs`)
   - Remove `#[allow(dead_code)]` attribute
   - Replace placeholder API calls with actual `AnthropicClient::create_message()` calls
   - Add file I/O callbacks for reading/writing code files
   - Implement progress callback to TUI via `EngineEvent::Status`
   - Handle streaming vs non-streaming responses

2. **Add file system integration** (`src/duo.rs`)
   - Create helper functions:
     - `read_file(path)` - read existing code files
     - `write_file(path, content)` - save generated code
     - `list_files(directory)` - explore workspace
     - `validate_path(path, workspace)` - security sandboxing
   - Implement file sandboxing (limit to workspace directory)

3. **Connect to existing client** (`src/duo.rs`)
   - Use `AnthropicClient` from `src/client.rs`
   - Leverage existing `MessageRequest` and `Message` types
   - Use coding model (`MiniMax-M2.1-Coding`)
   - Handle streaming vs non-streaming modes

4. **Progress reporting** (`src/core/engine.rs`)
   - Map progress callbacks to `EngineEvent::Status` messages
   - Display current phase (Player/Coach) in TUI footer
   - Show quality scores and feedback in status line
   - Integrate with existing session management

5. **Implement CLI handlers** (`src/main.rs`)
   - `DuoSubcommand::Start` - Parse requirements file, start workflow
   - `DuoSubcommand::Continue` - Resume session by ID
   - `DuoSubcommand::Sessions` - List saved sessions from disk
   - Proper error handling and user feedback

---

### Phase 2: Session Persistence

#### Tasks

1. **Session serialization** (`src/duo.rs`)
   - Add `save_session(session, path)` function
   - Add `load_session(path)` function
   - Use JSON serialization (serde already available)
   - Store in `~/.minimax/sessions/duo/`

2. **Session management**
   - Add `list_sessions()` to list saved sessions
   - Add `delete_session(session_id)` for cleanup
   - Support session resume by ID

3. **CLI integration**
   - `minimax duo sessions` - List sessions from disk
   - `minimax duo continue <id>` - Load and resume session
   - Auto-save sessions after each turn

---

### Phase 3: TUI View

#### Tasks

1. **Duo mode screen** (`src/tui/ui.rs`)
   - Add `render_duo_mode()` function
   - Display player-coach loop visualization
   - Show current phase with color coding
   - Display quality scores and progress

2. **Session browser** (`src/tui/session_picker.rs` or new file)
   - List saved Duo sessions
   - Show session metadata (status, phase, turns)
   - Select session to resume

3. **Progress indicator**
   - Show phase in footer: `🎮 Player Phase (Turn 2/10)`
   - Display coach feedback in modal
   - Show approval status with icons

---

### Phase 4: File System Integration

#### Tasks

1. **Code generation workflow**
   - Player writes code to workspace files
   - Coach reads files for validation
   - Implement diff-based updates

2. **File operations**
   - `read_file()` for coach validation
   - `write_file()` for player implementation
   - `list_directory()` for context
   - File filtering (skip `.git`, `target`, etc.)

3. **Security**
   - Path validation and sandboxing
   - File size limits
   - Approval for dangerous operations

---

## File Structure

```
 src/
 ├── duo.rs                    # State machine + workflow (extended with file I/O)
 ├── tools/duo.rs              # Tool definitions (already complete)
 ├── config.rs                 # Config (already complete)
 ├── core/
 │   ├── engine.rs            # Engine integration (extended for progress)
 │   ├── events.rs            # Event types
 │   └── ops.rs               # Operations
 ├── tui/
 │   ├── app.rs               # App state (already has Duo mode)
 │   ├── ui.rs                # Rendering (extended for Duo view)
 │   └── views/               # New: Duo view
 ├── main.rs                  # CLI handlers (extended with full implementation)
 └── prompts.rs               # System prompts (already has DUO_PROMPT)
```

---

## Key Design Decisions

1. **Reuse existing infrastructure**
   - Use `AnthropicClient` for API calls
   - Leverage `MessageRequest` and `Message` types
   - Use `EngineEvent::Status` for progress
   - Follow existing patterns in `src/rlm/`

2. **Security first**
   - File operations sandboxed to workspace
   - Path validation before all file I/O
   - No arbitrary file system access

3. **Progress reporting**
   - All phases reported via engine events
   - TUI can choose what to display
   - Consistent with other modes

4. **Session persistence**
   - JSON format for human readability
   - Store in standard config directory
   - Support resume by ID or prefix

---

## Acceptance Criteria

### Phase 1: Workflow Execution ✅ COMPLETE
- [x] `minimax duo start --requirements docs/requirements.md` works
- [x] Player phase generates code and advances to Coach
- [x] Coach phase validates and provides feedback
- [x] Loop continues until approval or timeout
- [x] Progress displayed in TUI status line
- [x] Code files can be written to workspace
- [x] Session auto-saves to disk
- [x] `minimax duo sessions` lists saved sessions
- [x] `minimax duo continue <id>` resumes session
- [x] Sessions persist across CLI invocations

---

## Testing Strategy

1. **Unit tests** (already exist - 24 tests in duo.rs)
   - State transitions
   - Prompt generation
   - Tool schemas

2. **Integration tests** (new)
   - Full workflow with mocked API
   - File I/O operations
   - CLI command execution

3. **E2E tests** (new)
   - Complete player-coach loop
   - Session persistence
   - TUI rendering

---

## Notes

- The `run_duo_workflow()` function signature uses async callbacks
- Need to handle both streaming and non-streaming API responses
- Coach must read implementation files to validate
- File operations must be thread-safe with the rest of the engine
