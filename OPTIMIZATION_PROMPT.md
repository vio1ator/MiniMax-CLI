# MiniMax CLI Optimization Prompt

Use this prompt with Claude, GPT-4, or another AI to improve MiniMax CLI based on patterns from OpenAI's Codex CLI.

---

## Context

I'm building **MiniMax CLI** - an unofficial Rust TUI for MiniMax M2.1 (similar to Claude Code). I've analyzed OpenAI's Codex CLI (`codex-rs`) and want to adopt their best patterns.

**Repositories:**
- MiniMax CLI: `/Volumes/VIXinSSD/minimax-cli/`
- Codex reference: `/Volumes/VIXinSSD/codex-main/codex-rs/`

## Recent Features Added (v0.1.7)

### 1. Duo Mode (Player-Coach Autocoding)
- **Files:** `src/duo.rs`, `src/tools/duo.rs`, `src/prompts/duo.txt`
- **Tools:** `duo_init`, `duo_player`, `duo_coach`, `duo_advance`, `duo_status`
- **Workflow:** init → player → coach → advance → (repeat until approved)

### 2. Bug Fixes
- **Approval flow:** Was sending tool_name instead of tool_use_id (`src/tui/approval.rs`)
- **Cursor sync:** Was using byte offset instead of char index (`src/tui/ui.rs`)

### 3. Character-Level Text Selection
- **File:** `src/tui/ui.rs` - `apply_selection()` and `apply_selection_to_line()`

---

## Patterns to Adopt from Codex

### Priority 1: Renderable Trait Abstraction

**Codex pattern** (`tui/src/render/renderable.rs`):
```rust
pub trait Renderable {
    fn render(&self, area: Rect, buf: &mut Buffer);
    fn desired_height(&self, width: u16) -> u16;
    fn cursor_pos(&self, _area: Rect) -> Option<(u16, u16)> { None }
}
```

**Task:** Create `src/tui/widgets/renderable.rs` with this trait. Convert major widgets (chat, composer, approval overlay) to implement it.

### Priority 2: View Stack for Modals

**Codex pattern** (`tui/src/bottom_pane/mod.rs`):
```rust
pub trait BottomPaneView {
    fn handle_key(&mut self, key: KeyEvent) -> CancellationEvent;
    fn render(&self, area: Rect, buf: &mut Buffer);
}

struct BottomPane {
    composer: ChatComposer,
    view_stack: Vec<Box<dyn BottomPaneView>>,
}
```

**Task:** Refactor approval overlay to use view stack pattern. Allow multiple modals (approval, file picker, help) to stack.

### Priority 3: Tool Runtime Trait

**Codex pattern** (`core/src/tools/spec.rs`):
```rust
pub trait ToolRuntime<Rq, Out> {
    fn exec_approval_requirement(&self, req: &Rq) -> Option<ExecApprovalRequirement>;
    fn sandbox_mode_for_first_attempt(&self, req: &Rq) -> SandboxOverride;
    async fn attempt(&mut self, req: &Rq, attempt: &SandboxAttempt) -> Result<Out, SandboxErr>;
}
```

**Task:** Move approval logic from TUI to tool layer. Each tool declares its own approval requirement.

### Priority 4: Parallel Tool Execution

**Codex pattern** (`core/src/tools/parallel.rs`):
```rust
let supports_parallel = self.router.tool_supports_parallel(&call.tool_name);
let _guard = if supports_parallel {
    Either::Left(lock.read().await)  // shared read lock
} else {
    Either::Right(lock.write().await)  // exclusive write lock
};
```

**Task:** Add `supports_parallel` to tool specs. File reads and searches can run in parallel; writes need exclusive lock.

### Priority 5: Event Broker for Terminal Handoff

**Codex pattern** (`tui/src/tui/event_stream.rs`):
```rust
pub struct EventBroker {
    state: Mutex<EventBrokerState>,
    resume_events_tx: watch::Sender<()>,
}

impl EventBroker {
    pub fn pause_events(&self) { /* drop crossterm stream */ }
    pub fn resume_events(&self) { /* recreate stream */ }
}
```

**Task:** Allow pausing/resuming terminal events when spawning external editors (vim, nano).

---

## Current MiniMax Architecture

### Key Files to Modify:
- `src/tui/ui.rs` (3300+ lines) - Main TUI loop, needs refactoring
- `src/tui/approval.rs` - Approval overlay (convert to view stack)
- `src/core/engine.rs` - Engine loop (add parallel execution)
- `src/tools/registry.rs` - Tool registry (add approval requirements)
- `src/tools/spec.rs` - Tool specs (add `supports_parallel`)

### Current Patterns:
- `HistoryCell` enum for chat history
- `TranscriptSelection` with anchor/head points
- `TranscriptScroll` state machine
- `EngineHandle` with channels for ops/events

---

## Specific Tasks

### Task 1: Create Renderable Trait
1. Create `src/tui/widgets/mod.rs` and `src/tui/widgets/renderable.rs`
2. Define `Renderable` trait with `render()`, `desired_height()`, `cursor_pos()`
3. Create `ChatWidget`, `ComposerWidget`, `ApprovalWidget` implementing trait
4. Refactor `render()` in `ui.rs` to use these widgets

### Task 2: Implement View Stack
1. Create `src/tui/views/mod.rs` with `ViewStack` struct
2. Define `ModalView` trait similar to `BottomPaneView`
3. Convert `ApprovalState` to `ApprovalView` implementing trait
4. Update key handling to route through view stack

### Task 3: Enhance Tool Specs
1. Add `ApprovalRequirement` enum to `src/tools/spec.rs`
2. Add `supports_parallel: bool` to `ToolSpec`
3. Move approval decision logic from `ui.rs` to tool layer
4. Update engine to check tool requirements

### Task 4: Add Parallel Execution
1. Add `RwLock` wrapper for tool execution context
2. Check `supports_parallel` before acquiring lock
3. Mark read-only tools (file_read, grep, glob) as parallel-safe
4. Test with concurrent file reads

### Task 5: Event Broker
1. Create `src/tui/event_broker.rs`
2. Implement pause/resume for crossterm events
3. Use in shell tool when spawning interactive processes
4. Prevent input stealing during subprocess execution

---

## Code Style

- Use conventional commits: `feat:`, `fix:`, `refactor:`
- Run `cargo fmt` and `cargo clippy --all-targets --all-features`
- Keep changes incremental and testable
- Prefer small PRs over large refactors

---

## Reference Files

**From Codex (patterns to follow):**
- `codex-rs/tui/src/render/renderable.rs`
- `codex-rs/tui/src/bottom_pane/mod.rs`
- `codex-rs/core/src/tools/orchestrator.rs`
- `codex-rs/core/src/tools/parallel.rs`
- `codex-rs/tui/src/tui/event_stream.rs`

**From MiniMax (files to modify):**
- `src/tui/ui.rs`
- `src/tui/approval.rs`
- `src/core/engine.rs`
- `src/tools/spec.rs`
- `src/tools/registry.rs`

---

## Success Criteria

1. TUI widgets are composable via `Renderable` trait
2. Modals stack properly (approval + help can coexist)
3. Tool approval logic lives in tool layer, not TUI
4. File reads/searches run in parallel
5. External editor spawning works without input issues
6. All tests pass, clippy clean, formatted

Please start with Task 1 (Renderable trait) and work through incrementally.
