# AGENTS.md — Coding Agent Session Search (cass)

> Guidelines for AI coding agents working in this Rust codebase.

---

## RULE NUMBER 1: NO FILE DELETION

**YOU ARE NEVER ALLOWED TO DELETE A FILE WITHOUT EXPRESS PERMISSION.** Even a new file that you yourself created, such as a test code file. You have a horrible track record of deleting critically important files or otherwise throwing away tons of expensive work. As a result, you have permanently lost any and all rights to determine that a file or folder should be deleted.

**YOU MUST ALWAYS ASK AND RECEIVE CLEAR, WRITTEN PERMISSION BEFORE EVER DELETING A FILE OR FOLDER OF ANY KIND.**

---

## Irreversible Git & Filesystem Actions — DO NOT EVER BREAK GLASS

1. **Absolutely forbidden commands:** `git reset --hard`, `git clean -fd`, `rm -rf`, or any command that can delete or overwrite code/data must never be run unless the user explicitly provides the exact command and states, in the same message, that they understand and want the irreversible consequences.
2. **No guessing:** If there is any uncertainty about what a command might delete or overwrite, stop immediately and ask the user for specific approval. "I think it's safe" is never acceptable.
3. **Safer alternatives first:** When cleanup or rollbacks are needed, request permission to use non-destructive options (`git status`, `git diff`, `git stash`, copying to backups) before ever considering a destructive command.
4. **Mandatory explicit plan:** Even after explicit user authorization, restate the command verbatim, list exactly what will be affected, and wait for a confirmation that your understanding is correct. Only then may you execute it—if anything remains ambiguous, refuse and escalate.
5. **Document the confirmation:** When running any approved destructive command, record (in the session notes / final response) the exact user text that authorized it, the command actually run, and the execution time. If that record is absent, the operation did not happen.

---

## Toolchain: Rust & Cargo

We only use **Cargo** in this project, NEVER any other package manager.

- **Edition:** Rust 2024 (nightly)
- **Dependency versions:** Wildcard constraints (`*`) for all crates
- **Configuration:** Cargo.toml only

Follow best practices from `RUST_BEST_PRACTICES_GUIDE.md`.

### Environment Variables

We load all configuration from `.env` via the **dotenvy** crate. NEVER use `std::env::var()` directly.

```rust
use dotenvy::dotenv;
use std::env;

// Load .env file at startup (typically in main())
dotenv().ok();

// Configuration with fallback
let api_base_url = env::var("API_BASE_URL")
    .unwrap_or_else(|_| "http://localhost:8007".to_string());
```

The `.env` file exists and **MUST NEVER be overwritten**.

---

## Database Guidelines (sqlx/rusqlite)

**Do:**
- Create connection pools with `sqlx::Pool::connect()` and reuse across the application
- Use `?` placeholders for parameters (prevents SQL injection)
- Use query macros (`query!`, `query_as!`) for compile-time verification
- Keep one database transaction per logical operation
- Use `fetch_one()`, `fetch_optional()`, or `fetch_all()` appropriately
- Handle migrations with sqlx-cli: `sqlx migrate run`
- Use strong typing with `sqlx::types` for custom database types

**Don't:**
- Share a single transaction across concurrent tasks
- Use string concatenation to build SQL queries
- Ignore `Option<T>` for nullable columns
- Mix sync and async database operations
- Use `unwrap()` on database results in production code

---

## Code Editing Discipline

### No Script-Based Changes

**NEVER** run a script that processes/changes code files in this repo. Brittle regex-based transformations create far more problems than they solve.

- **Always make code changes manually**, even when there are many instances
- For many simple changes: use parallel subagents
- For subtle/complex changes: do them methodically yourself

### No File Proliferation

If you want to change something or add a feature, **revise existing code files in place**.

**NEVER** create variations like:
- `document_processorV2.rs`
- `document_processor_improved.rs`
- `document_processor_enhanced.rs`

New files are reserved for **genuinely new functionality** that makes zero sense to include in any existing file. The bar for creating new files is **incredibly high**.

---

## Backwards Compatibility

We do not care about backwards compatibility—we're in early development with no users. We want to do things the **RIGHT** way with **NO TECH DEBT**.

- Never create "compatibility shims"
- Never create wrapper functions for deprecated APIs
- Just fix the code directly

---

## Console Output Style

All console output should be **informative, detailed, stylish, and colorful** by leveraging:
- `colored` — ANSI color formatting
- `indicatif` — Progress bars and spinners
- `console` — Terminal utilities

---

## Compiler Checks (CRITICAL)

**After any substantive code changes, you MUST verify no errors were introduced:**

```bash
# Check for compiler errors and warnings
cargo check --all-targets

# Check for clippy lints
cargo clippy --all-targets -- -D warnings

# Verify formatting
cargo fmt --check
```

If you see errors, **carefully understand and resolve each issue**. Read sufficient context to fix them the RIGHT way.

---

## Third-Party Library Usage

If you aren't 100% sure how to use a third-party library, **SEARCH ONLINE** to find the latest documentation and mid-2025 best practices.

---

## cass — Coding Agent Session Search

**This is the project you're working on.** cass indexes conversations from Claude Code, Codex, Cursor, Gemini, Aider, ChatGPT, and more into a unified, searchable index.

**NEVER run bare `cass`** — it launches an interactive TUI. Always use `--robot` or `--json`.

### Quick Start

```bash
# Check if index is healthy (exit 0=ok, 1=run index first)
cass health

# Search across all agent histories
cass search "authentication error" --robot --limit 5

# View a specific result (from search output)
cass view /path/to/session.jsonl -n 42 --json

# Expand context around a line
cass expand /path/to/session.jsonl -n 42 -C 3 --json

# Learn the full API
cass capabilities --json      # Feature discovery
cass robot-docs guide         # LLM-optimized docs
```

### Key Flags

| Flag | Purpose |
|------|---------|
| `--robot` / `--json` | Machine-readable JSON output (required!) |
| `--fields minimal` | Reduce payload: `source_path`, `line_number`, `agent` only |
| `--limit N` | Cap result count |
| `--agent NAME` | Filter to specific agent (claude, codex, cursor, etc.) |
| `--days N` | Limit to recent N days |

**stdout = data only, stderr = diagnostics. Exit 0 = success.**

### Robot Mode Etiquette

- Prefer `cass --robot-help` and `cass robot-docs <topic>` for machine-first docs
- The CLI is forgiving: globals placed before/after subcommand are auto-normalized
- If parsing fails, follow the actionable errors with examples
- Use `--color=never` in non-TTY automation for ANSI-free output

### Auto-Correction Features

| Mistake | Correction | Note |
|---------|------------|------|
| `-robot` | `--robot` | Long flags need double-dash |
| `--Robot`, `--LIMIT` | `--robot`, `--limit` | Flags are lowercase |
| `find "query"` | `search "query"` | `find` is an alias |
| `--robot-docs` | `robot-docs` | It's a subcommand |

**Full alias list:**
- **Search:** `find`, `query`, `q`, `lookup`, `grep` → `search`
- **Stats:** `ls`, `list`, `info`, `summary` → `stats`
- **Status:** `st`, `state` → `status`
- **Index:** `reindex`, `idx`, `rebuild` → `index`
- **View:** `show`, `get`, `read` → `view`
- **Robot-docs:** `docs`, `help-robot`, `robotdocs` → `robot-docs`

### Pre-Flight Health Check

```bash
cass health --json
```

Returns in <50ms:
- **Exit 0:** Healthy—proceed with queries
- **Exit 1:** Unhealthy—run `cass index --full` first

### Exit Codes

| Code | Meaning | Retryable |
|------|---------|-----------|
| 0 | Success | N/A |
| 1 | Health check failed | Yes—run `cass index --full` |
| 2 | Usage/parsing error | No—fix syntax |
| 3 | Missing index | Yes—run `cass index` first |
| 9 | Unknown error | Maybe |

---

## Morph Warp Grep — AI-Powered Code Search

**Use `mcp__morph-mcp__warp_grep` for exploratory "how does X work?" questions.** An AI agent expands your query, greps the codebase, reads relevant files, and returns precise line ranges with full context.

**Use `ripgrep` for targeted searches.** When you know exactly what you're looking for.

**Use `ast-grep` for structural patterns.** When you need AST precision for matching/rewriting.

### When to Use What

| Scenario | Tool | Why |
|----------|------|-----|
| "How is authentication implemented?" | `warp_grep` | Exploratory; don't know where to start |
| "Where is rate limiting implemented?" | `warp_grep` | Need to understand architecture |
| "Find all uses of `embed(`" | `ripgrep` | Targeted literal search |
| "Find files with `println!`" | `ripgrep` | Simple pattern |
| "Replace all `unwrap()` with `expect()`" | `ast-grep` | Structural refactor |

### warp_grep Usage

```
mcp__morph-mcp__warp_grep(
  repoPath: "/path/to/cass",
  query: "How is semantic search implemented?"
)
```

Returns structured results with file paths, line ranges, and extracted code snippets.

### Anti-Patterns

- **Don't** use `warp_grep` to find a specific function name → use `ripgrep`
- **Don't** use `ripgrep` to understand "how does X work" → wastes time with manual reads
- **Don't** use `ripgrep` for codemods → risks collateral edits
