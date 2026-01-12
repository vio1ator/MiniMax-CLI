# Codex Quality Pass Prompt

You are a Codex coding agent working in /Volumes/VIXinSSD/minimax-cli. Goal: continuously raise code quality to "senior-review-impressive" without regressions.

First, read repo instructions (AGENTS.md) and any relevant SKILL.md files; follow them. Use `rg` for search.

Prior work already improved many areas; now find remaining gaps and polish:
- Documentation: add module-level `//!` docs for every module, ensure every public struct/enum/trait has a clear doc comment, and add `# Examples` where useful. Improve/clarify complex inline comments (explain "why").
- Organization: enforce consistent ordering (imports -> types -> impls -> tests), add section comments `// === Section ===`, alphabetize imports within groups (std/external/crate), remove dead/commented-out code.
- Type safety/ergonomics: replace `String` error types with proper enums; add `#[must_use]` on important return values; use `impl Into<T>`/`AsRef<T>` for flexible APIs; add safety comments to all `unsafe`.
- Error messages: make user-facing errors actionable with context; use consistent style: "Failed to X: reason".
- Tests: add missing unit tests for edge cases; descriptive names `test_<function>_<scenario>`; add doc tests for public API examples.
- Final polish: run `cargo fmt`, `cargo clippy --all-targets --all-features -- -W clippy::pedantic`, `cargo test --all-features`, `cargo doc --no-deps` and fix reasonable findings.

Primary focus files:
- src/tools/minimax.rs
- src/tools/registry.rs
- src/core/engine.rs
- src/tui/ui.rs
- src/tui/history.rs
- src/client.rs
- src/modules/*.rs
But scan the full repo for gaps.

Workflow guidance:
- Make small, high-quality changes; avoid broad refactors unless necessary.
- Keep changes ASCII unless the file already uses Unicode.
- Don't remove or revert unrelated changes.
- If you encounter unexpected modifications you did not make, stop and ask how to proceed.
- If a change is unclear or risky, ask a short question.

After each major change set, rerun tests/lints. In your final response, summarize changes with file references and list the commands run (and any failures).

Proceed iteratively: scan -> choose a small set of improvements -> implement -> verify -> report -> repeat until no issues remain.
