# Comprehensive Test Plan for minimax-cli

## Current State (as of v0.6.0)

### Test Coverage
- ✅ 1 test file: `tests/palette_audit.rs` (color palette validation)
- ❌ No unit tests for core functionality
- ❌ No integration tests for workflows
- ❌ No API mocking in tests

### Key Modules Without Tests
- Config loading and parsing
- Session state management
- Core engine orchestration
- All tool implementations
- Feature flags
- RLM context handling
- Duo state machine (only 24 unit tests exist)

---

## Testing Strategy

### Approach: Integration-First with Unit Coverage

1. **Start with integration tests** for critical workflows
2. **Add unit tests** for individual functions/types
3. **Use property-based testing** for state machines
4. **Add API mocking** with wiremock for HTTP interactions

---

## Test Structure

```
tests/
├── common.rs                 # Test utilities (temp dirs, fixtures)
├── config_tests.rs           # Config loading & parsing
├── session_tests.rs          # Session state management
├── engine_tests.rs           # Core engine orchestration
├── features_tests.rs         # Feature flags
├── workspace_tests.rs        # Path safety & sandboxing
├── utils_tests.rs            # Helper functions
├── palette_audit.rs          # Color audit (existing)
├── tools/
│   ├── file_tests.rs         # File operations
│   ├── shell_tests.rs        # Shell execution
│   ├── web_search_tests.rs   # Web tools
│   ├── subagent_tests.rs     # Subagent tools
│   ├── memory_tests.rs       # Memory tools
│   └── ...
├── state_machines/
│   ├── duo_state_tests.rs    # Duo state machine
│   └── rlm_context_tests.rs  # RLM context
└── e2e_tests.rs              # End-to-end workflows
```

---

## Phase 1: Core Module Tests

### 1.1 Config Module (`tests/config_tests.rs`)

**Coverage**:
- Config loading from file
- Config loading from environment variables
- Profile selection
- API key resolution
- Default value application
- Feature flag parsing
- Retry policy calculation

**Key Scenarios**:
```rust
// Test cases:
- Missing config file → fallback to defaults
- Invalid TOML → proper error
- Environment variable overrides
- Profile not found → error
- Multiple profiles in one config
```

---

### 1.2 Session Module (`tests/session_tests.rs`)

**Coverage**:
- Session creation with/without project context
- Message history management
- Token usage tracking
- Session persistence
- Pinned messages
- Project context loading

**Key Scenarios**:
```rust
// Test cases:
- New session initialization
- Adding messages to history
- Usage aggregation
- Project context from AGENTS.md
- Empty workspace (no context)
```

---

### 1.3 Engine Module (`tests/engine_tests.rs`)

**Coverage**:
- Tool registry construction
- Tool execution orchestration
- Feature flag filtering
- Approval workflows
- Parallel tool execution
- Context compaction triggers

**Key Scenarios**:
```rust
// Test cases:
- Registering tools
- Looking up tools by name
- Feature-gated tool visibility
- Parallel vs sequential execution
- Compaction thresholds
```

---

### 1.4 Feature Flags (`tests/features_tests.rs`)

**Coverage**:
- Feature activation/deactivation
- Default feature set
- Feature dependencies
- Unknown feature keys

**Key Scenarios**:
```rust
// Test cases:
- Enabling/disabling features
- Feature-dependent tool availability
- Invalid feature names → graceful handling
```

---

## Phase 2: Tool Implementation Tests

### 2.1 File Tools (`tests/tools/file_tests.rs`)

**Coverage**:
- `ReadFileTool`: valid files, missing files, path traversal prevention
- `WriteFileTool`: valid writes, path validation, directory creation
- `EditFileTool`: search/replace, no matches, empty search
- `ListDirTool`: valid dirs, empty dirs, permissions

**Key Scenarios**:
```rust
// Test cases:
- Path traversal attacks → blocked
- Workspace boundary enforcement
- Non-existent files → errors
- Empty content handling
- Permission errors
```

---

### 2.2 Shell Tools (`tests/tools/shell_tests.rs`)

**Coverage**:
- `ExecShellTool`: successful commands, failed commands
- Shell execution with/without approval
- Timeout handling
- Output capture

**Key Scenarios**:
```rust
// Test cases:
- Valid commands (echo, pwd)
- Invalid commands (exit 1)
- Shell execution disabled → blocked
- Long-running commands → timeout
```

---

### 2.3 Web Search Tools (`tests/tools/web_search_tests.rs`)

**Coverage**:
- `WebSearchTool`: mock HTTP responses
- `WebFetchTool`: HTML parsing
- Rate limiting handling

**Key Scenarios**:
```rust
// Test cases:
- Successful search
- No results
- Network errors
- HTML parsing edge cases
```

---

### 2.4 Subagent Tools (`tests/tools/subagent_tests.rs`)

**Coverage**:
- Subagent creation
- Concurrency limits
- Resource cleanup

---

### 2.5 Memory Tools (`tests/tools/memory_tests.rs`)

**Coverage**:
- Save/retrieve memory entries
- Memory persistence
- Query filtering

---

## Phase 3: State Machine Tests (Property-Based)

### 3.1 Duo State Machine (`tests/state_machines/duo_state_tests.rs`)

**Coverage**:
- Phase transitions (Init → Player → Coach → Approved)
- Invalid transitions rejected
- Max turns reached (timeout)
- Quality score calculation

**Approach**: Use `proptest` for property-based testing

**Key Properties**:
```rust
// Test properties:
- Cannot advance from Approved/Timeout
- Valid phase sequence enforcement
- Turn counting accuracy
- Quality score averaging
- Session persistence roundtrip
```

**Example Tests**:
```rust
// Property: Valid phase sequence
proptest! {
    #[test]
    fn valid_phase_sequence(state in duo_state_strategy()) {
        // State machine should only allow valid transitions
    }
}

// Property: Turn counting
proptest! {
    #[test]
    fn turn_counting(state in duo_state_strategy()) {
        // Turn counter must match history length
    }
}
```

---

### 3.2 RLM Context (`tests/state_machines/rlm_context_tests.rs`)

**Coverage**:
- Context loading from files
- Search with regex
- Chunking with overlap
- Variable storage/retrieval

**Key Scenarios**:
```rust
// Test cases:
- Large file chunking
- Regex search edge cases
- Context overflow handling
- Variable persistence
```

---

## Phase 4: Integration Tests

### 4.1 Workspace Safety (`tests/workspace_tests.rs`)

**Coverage**:
- Path resolution within workspace
- Absolute path rejection (outside workspace)
- Trust mode bypass
- File operations outside workspace → blocked

---

### 4.2 End-to-End Tests (`tests/e2e_tests.rs`)

**Coverage**:
- Full chat session (user prompt → AI response)
- Tool execution workflow (AI → tool call → result)
- Multi-turn conversations with context
- Session resume from saved state

**Note**: May need wiremock for API mocking

---

## Test Infrastructure

### Common Utilities (`tests/common.rs`)

```rust
// Available functions:
- temp_workspace() → create temp directory for tests
- mock_config() → create test config
- fixture_path() → load test fixtures
- assert_json_snapshot() → JSON comparison
- test_client() → HTTP client for tests
- write_test_file() → create test file
- read_test_file() → read test file
```

---

## Implementation Schedule

### Week 1: Core Modules
- [ ] Config tests
- [ ] Session tests
- [ ] Utils tests
- [ ] Common utilities

### Week 2: Tool Tests
- [ ] File tools tests
- [ ] Shell tools tests
- [ ] Web search tools tests
- [ ] Subagent tests

### Week 3: Feature Flags & Engine
- [ ] Feature flags tests
- [ ] Engine integration tests
- [ ] Workspace safety tests

### Week 4: State Machines
- [ ] Duo state machine tests (property-based)
- [ ] RLM context tests

### Week 5: E2E & Coverage
- [ ] End-to-end tests
- [ ] Test coverage reporting
- [ ] CI integration

---

## Coverage Goals

| Module | Target Coverage | Test Type |
|--------|----------------|-----------|
| Config | 90% | Integration |
| Session | 85% | Integration |
| Engine | 80% | Integration |
| Tools | 85% | Unit + Integration |
| Features | 95% | Unit |
| Utils | 90% | Unit |
| RLM/Duo | 75% | Property-based |
| E2E | 70% | Integration |

---

## Tools & Dependencies

### Existing Dev Dependencies
- `pretty_assertions` - Better assertion messages
- `wiremock` - HTTP server mocking

### New Dependencies (to add)
```toml
[dev-dependencies]
proptest = "1.0"          # Property-based testing
tempfile = "3.16"         # Temp directory management
insta = "1.0"             # Snapshot testing (optional)
```

---

## CI Integration

### Add to `.github/workflows/ci.yml`
```yaml
test-coverage:
  name: Test Coverage
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        toolchain: stable
        components: llvm-tools-preview
    - uses: Swatinem/rust-cache@v2
    - name: Install cargo-llvm-cov
      run: cargo install cargo-llvm-cov
    - name: Generate coverage
      run: cargo llvm-cov --all-features --html
    - name: Upload coverage
      uses: actions/upload-artifact@v4
      with:
        name: coverage-report
        path: target/llvm-cov/html
```

---

## Success Metrics

### Minimum Viable Test Suite
- ✅ 100+ unit/integration tests
- ✅ Config, Session, Engine coverage
- ✅ Core tools tested
- ✅ Duo state machine property tests
- ✅ CI pipeline passing

### stretch Goals
- 🎯 80%+ code coverage
- 🎯 Property-based tests for state machines
- 🎯 Snapshot testing for UI outputs
- 🎯 E2E test suite

---

## Notes

- Start with integration tests for critical paths
- Add unit tests for complex logic
- Use wiremock for API mocking
- Property-based tests for state machines
- Test workspace boundary security rigorously
- Ensure all tools test approval workflows
