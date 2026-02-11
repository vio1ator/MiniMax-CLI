# Comprehensive Plan: Remove MiniMax Features & Rename to Axiom

## Overview

Replace all MiniMax-specific features with a generic abstraction layer, remove multimedia tools, and rename the project to **Axiom**.

## Scope

### Keep (as requested)
- ✅ **Duo mode** - Player-coach paradigm
- ✅ **RLM mode** - Externalized context
- ✅ **Tool infrastructure** - Registry, spec, approval system
- ✅ **TUI** - Complete terminal interface
- ✅ **File tools** - Read/write/patch
- ✅ **Shell tools** - Command execution
- ✅ **Git tools** - Version control
- ✅ **Web search** - Public APIs
- ✅ **MCP** - External tool servers
- ✅ **Subagents** - Multi-agent orchestration

### Remove
- ❌ All MiniMax API calls
- ❌ TTS/Text-to-Speech
- ❌ Image generation
- ❌ Video generation
- ❌ Music generation
- ❌ Voice cloning
- ❌ File upload/retrieve from MiniMax

---

## Phase 1: Project Renaming & Branding

### 1.1 Package & Binary
- **Package name:** `minimax-cli` → `axiom-cli`
- **Binary name:** `minimax` → `axiom`
- **Config directory:** `~/.axiom/` → `~/.axiom/`

**Files to update:**
- `Cargo.toml` - package name, binary name
- `src/main.rs` - CLI name, help text
- `README.md` - all references
- `config.example.toml` - comments
- `.github/workflows/*.yml` - CI/CD

### 1.2 Environment Variables
- `MINIMAX_API_KEY` → `AXIOM_API_KEY`
- `MINIMAX_BASE_URL` → `AXIOM_BASE_URL`
- `MINIMAX_API_KEY_2` → `AXIOM_API_KEY_2`
- `MINIMAX_BASE_URL_2` → `AXIOM_BASE_URL_2`
- `MINIMAX_PROFILE` → `AXIOM_PROFILE`
- `MINIMAX_CONFIG_PATH` → `AXIOM_CONFIG_PATH`
- `MINIMAX_MCP_CONFIG` → `AXIOM_MCP_CONFIG`
- `MINIMAX_SKILLS_DIR` → `AXIOM_SKILLS_DIR`
- `MINIMAX_NOTES_PATH` → `AXIOM_NOTES_PATH`
- `MINIMAX_MEMORY_PATH` → `AXIOM_MEMORY_PATH`
- `MINIMAX_ALLOW_SHELL` → `AXIOM_ALLOW_SHELL`
- `MINIMAX_MAX_SUBAGENTS` → `AXIOM_MAX_SUBAGENTS`
- `MINIMAX_MODEL_CONTEXT_WINDOWS` → `AXIOM_MODEL_CONTEXT_WINDOWS`

---

## Phase 2: Generic API Abstraction Layer

### 2.1 Create Provider System
**New file:** `src/provider.rs`

```rust
// Generic trait for any LLM provider
trait LlmProvider {
    fn provider_name(&self) -> &'static str;
    fn model(&self) -> &str;
    fn create_message(&self, request: MessageRequest) -> Result<MessageResponse>;
    fn create_message_stream(&self, request: MessageRequest) -> Result<StreamEventBox>;
}
```

### 2.2 Refactor Clients
- **Keep:** `AnthropicClient` (already generic-compatible)
- **Create:** `GenericProvider` for OpenAI/compatible APIs
- **Remove:** `MiniMaxClient`, `MiniMaxCodingClient`

**Files to update:**
- `src/client.rs` - Remove MiniMax-specific clients
- `src/llm_client.rs` - Update trait bounds
- `src/core/engine.rs` - Use provider abstraction

---

## Phase 3: Tool System Cleanup

### 3.1 Keep (Provider-Independent Tools)
- `src/tools/file.rs` - Read/write/patch
- `src/tools/shell.rs` - Command execution
- `src/tools/git.rs` - Git operations
- `src/tools/web_search.rs` - Web tools
- `src/tools/memory.rs` - Memory management
- `src/tools/artifact.rs` - Artifacts
- `src/tools/plan.rs` - Plan management
- `src/tools/todo.rs` - Todo list
- `src/tools/subagent.rs` - Subagents
- `src/tools/rlm.rs` - RLM operations
- `src/tools/spec.rs` - Tool specification

### 3.2 Remove (MiniMax-Dependent Tools)
**Delete entire file:** `src/tools/minimax.rs`

**Tools to remove:**
- `tts`, `tts_async_create`, `tts_async_query`
- `generate_image`, `analyze_image`
- `generate_video`, `query_video`, `generate_video_template`, `query_video_template`
- `generate_music`
- `upload_file`, `list_files`, `retrieve_file`, `download_file`, `delete_file`
- `voice_clone`, `voice_list`, `voice_delete`, `voice_design`

### 3.3 Update Tool Registry
**File:** `src/tools/registry.rs`
- Remove `with_minimax_tools()` method
- Remove minimax tools from re-exports
- Update documentation

**File:** `src/tools/mod.rs`
- Remove `minimax` module from pub mod
- Remove minimax tool re-exports

---

## Phase 4: Multimedia Module Cleanup

### 4.1 Remove Modules
**Delete entire files:**
- `src/modules/audio.rs`
- `src/modules/video.rs`
- `src/modules/image.rs`
- `src/modules/music.rs`
- `src/modules/files.rs` (if MiniMax-specific)

**Update:** `src/modules/mod.rs`
- Remove removed modules from pub mod

### 4.2 Update References
- `src/tools/registry.rs` - Remove minimax tools
- `src/llm_client.rs` - Remove any multimedia dependencies

---

## Phase 5: Configuration Updates

### 5.1 Remove MiniMax Defaults
**File:** `src/config.rs`
- Change `default_text_model` → `default_model`
- Remove MiniMax-specific base_url defaults
- Update `minimax_api_key()` → `api_key()`
- Update `minimax_base_url()` → `base_url()`
- Keep `coding_api_key()` and `coding_base_url()` for backwards compatibility
- Update comments to remove MiniMax references

**File:** `config.example.toml`
- Replace `MiniMax-M2.1` with generic model names
- Remove MiniMax-specific comments
- Update API key comments
- Update base_url comments

### 5.2 Model Resolution
- Default to first model in config file
- Support multiple model configurations
- No MiniMax-specific model logic

---

## Phase 6: UI/Color Palette Updates

### 6.1 Rename Palette Constants
**File:** `src/palette.rs`
- `MINIMAX_BLUE` → `BLUE`
- `MINIMAX_RED` → `RED`
- `MINIMAX_GREEN` → `GREEN`
- `MINIMAX_YELLOW` → `YELLOW`
- `MINIMAX_ORANGE` → `ORANGE`
- `MINIMAX_MAGENTA` → `MAGENTA`
- `MINIMAX_INK` → `INK`
- `MINIMAX_BLACK` → `BLACK`
- `MINIMAX_SLATE` → `SLATE`
- `MINIMAX_SILVER` → `SILVER`
- `MINIMAX_SNOW` → `SNOW`

### 6.2 Update UI References
**Files to update:**
- `src/tui/widgets/header.rs` - Update all MINIMAX_ references
- `src/tui/ui.rs` - Update colors, messages
- `src/tui/app.rs` - Update welcome messages
- `src/tui/model_picker.rs` - Update model descriptions
- `src/tui/search_view.rs` - Update labels
- `src/tui/tutorial.rs` - Update welcome text
- `src/main.rs` - Update help text, colors

**Change welcome messages:**
- "Welcome to MiniMax" → "Welcome to Axiom"
- "MiniMax M2.1" → "Model"
- Remove MiniMax-specific instructions

---

## Phase 7: Documentation Updates

### 7.1 README.md
- Rename project to Axiom
- Update all MiniMax references
- Update installation instructions
- Update configuration examples
- Update model names
- Update API key instructions

### 7.2 Documentation Files
**Files to update:**
- `docs/README.md`
- `docs/CONFIGURATION.md`
- `docs/DUO.md`
- `docs/RLM.md`
- `docs/MODES.md`
- `docs/ARCHITECTURE.md`
- `docs/MCP.md`

**Changes:**
- Replace "MiniMax CLI" with "Axiom CLI"
- Replace "MiniMax" with generic terms
- Update example API keys
- Update model examples

### 7.3 Code Comments
- Update all `//!` doc comments
- Remove MiniMax-specific references
- Update URLs

---

## Phase 8: Testing ✅ COMPLETED

### 8.1 Add Test Infrastructure ✅
**New file:** `tests/common.rs`
- Test utilities (temp dirs, fixtures)
- Mock configuration
- Workspace helpers

### 8.2 Add Unit Tests ✅
**New files:**
- `tests/config_tests.rs` - Config loading (basic tests)
- `tests/engine_tests.rs` - Engine orchestration
- `tests/features_tests.rs` - Feature flags
- `tests/tools/file_tests.rs` - File operations
- `tests/tools/shell_tests.rs` - Shell execution
- `tests/tools/git_tests.rs` - Git operations
- `tests/tools/memory_tests.rs` - Memory
- `tests/tools/subagent_tests.rs` - Subagents
- `tests/workspace_tests.rs` - Safety tests

**Note:** Tests are basic integration tests. Full unit tests requiring internal module exports need a `lib.rs` file.

### 8.3 Update CI ✅
**File:** `.github/workflows/ci.yml`
- Add test-coverage job
- Install llvm-cov
- Generate HTML reports

---

## Implementation Order

### Week 1: Core Refactoring
- [ ] Phase 1: Package renaming
- [ ] Phase 2: Provider abstraction
- [ ] Phase 3: Remove minimax tools
- [ ] Phase 4: Remove multimedia modules
- [ ] Phase 5: Configuration updates

### Week 2: UI Updates
- [ ] Phase 6: Palette and UI updates
- [ ] Phase 7: Documentation updates
- [ ] Fix all broken imports
- [ ] Build and fix compilation errors

### Week 3: Testing
- [x] Phase 8: Add test infrastructure
- [x] Add unit tests for core modules
- [x] Add tool tests
- [ ] Add state machine tests
- [x] Update CI pipeline

### Week 4: Finalization
- [ ] Update README and docs
- [ ] Fix remaining issues
- [x] Run cargo clippy
- [x] Run cargo fmt
- [ ] Test with sample configs
- [ ] Create migration guide

---

## Verification Checklist

### Build & Run
- [x] `cargo build` succeeds
- [x] `cargo test` passes (all new tests)
- [x] `cargo fmt` applied
- [x] `cargo clippy` has no warnings (pre-existing warnings only)
- [ ] Binary runs without MiniMax references

### Functionality
- [ ] Duo mode works
- [ ] RLM mode works
- [ ] File tools work
- [ ] Shell tools work
- [ ] Git tools work
- [ ] Web search works
- [ ] TUI renders correctly
- [ ] Configuration loads
- [ ] Session management works

### No MiniMax Left Behind
- [ ] No `MINIMAX_` environment variables
- [ ] No `api.minimax.io` URLs
- [ ] No `MiniMax` in code
- [ ] No `minimax` in paths
- [ ] No MiniMax-specific model names

---

## Breaking Changes

### Configuration
- `.minimax/config.toml` → `.axiom/config.toml`
- Rename environment variables
- Remove minimax-specific config keys

### Tools
- Remove: `tts`, `generate_image`, `generate_video`, `generate_music`, etc.
- Tools must now use generic API providers

### Models
- Remove `MiniMax-M2.1`, `MiniMax-M2.1-Coding` defaults
- Models must be configured explicitly

---

## Notes

- Keep Duo and RLM modes provider-agnostic
- Keep tool infrastructure working without MiniMax APIs
- Use generic provider abstraction for future extensibility
- Preserve all existing functionality except MiniMax-specific features
- Update all documentation and examples

---

## Post-Migration Tasks

1. **Deprecate MiniMax-specific features in README**
2. **Create migration guide** for users
3. **Update GitHub repository** description
4. **Update crates.io** package metadata
5. **Announce changes** in appropriate channels
6. **Archive old documentation** for reference