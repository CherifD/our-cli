# our-cli

`our-cli` is a Rust twin of `asm-agent`: a cross-platform CLI that talks to OpenAI, remembers local conversation history, prints token usage, and supports configurable terminal colors.

## Requirements

- macOS or Windows
- Rust toolchain
- OpenAI API key

## Build

```sh
cargo build
```

## Check

```sh
sh scripts/check.sh
```

The check script runs:

```sh
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

## CI

GitHub Actions runs on every pull request to `main` and every push to `main`. CI is split into separate macOS and Windows checks so failures are easy to diagnose:

| Check | Command |
| --- | --- |
| `format (macOS)` / `format (Windows)` | `cargo fmt -- --check` |
| `clippy (macOS)` / `clippy (Windows)` | `cargo clippy --all-targets -- -D warnings` |
| `test (macOS)` / `test (Windows)` | `cargo test` |
| `build (macOS)` / `build (Windows)` | `cargo build` |

The `main` branch is protected. Pull requests must be up to date and pass all required macOS and Windows checks before they can be merged. Merged PR branches are deleted automatically.

## Run

```sh
export OPENAI_API_KEY="your_api_key"
cargo run -- "explain Rust ownership in two sentences"
cargo run -- "what was my previous question?"
```

On Windows PowerShell:

```powershell
$env:OPENAI_API_KEY = "your_api_key"
cargo run -- "explain Rust ownership in two sentences"
cargo run -- "what was my previous question?"
```

## Commands

```sh
our-cli "your prompt"
our-cli chat
our-cli history
our-cli reset
our-cli --help
```

In `our-cli chat`, enter or paste one or more lines, then press return on a blank line to send the message. Type `/exit` or `/quit` as the first line of a message to leave chat.

## Local Install

```sh
cargo install --path .
our-cli "hello"
```

After publishing the repo to GitHub, another macOS or Windows machine with Rust installed can install it directly:

```sh
cargo install --git https://github.com/CherifD/our-cli.git
our-cli "hello from GitHub"
```

## Memory

Conversation memory is saved as plain text:

```text
~/.config/our-cli/conversation.txt
```

On Windows, the config directory comes from the operating system, usually:

```text
%APPDATA%\our-cli\conversation.txt
```

Use:

```sh
our-cli history
our-cli reset
```

## Colors

Defaults:

```sh
OUR_CLI_PROMPT_COLOR=d8ae6dfc
OUR_CLI_ASSISTANT_COLOR=00ffff
```

Override with 6- or 8-digit hex:

```sh
OUR_CLI_PROMPT_COLOR=8a2be2 our-cli chat
OUR_CLI_ASSISTANT_COLOR=ffcc00 our-cli "hello"
```

Disable color:

```sh
NO_COLOR=1 our-cli "hello"
OUR_CLI_COLOR=never our-cli chat
```

## Environment

| Variable | Purpose |
| --- | --- |
| `OPENAI_API_KEY` | API key used for requests |
| `AI_API_KEY` | Alternate API key variable |
| `OPENAI_MODEL` | Optional model override |
| `OPENAI_BASE_URL` | Optional OpenAI-compatible base URL |
| `OPENAI_MAX_OUTPUT_TOKENS` | Optional response length cap |
| `OUR_CLI_INSTRUCTIONS` | Optional assistant instructions |
| `OUR_CLI_STATE` | Optional memory file path |
| `OUR_CLI_MAX_HISTORY_LINES` | Optional max saved transcript lines |
| `OUR_CLI_COLOR` | Set to `never` to disable color |
| `OUR_CLI_PROMPT_COLOR` | Optional prompt/input color as hex |
| `OUR_CLI_ASSISTANT_COLOR` | Optional assistant response color as hex |
| `OUR_CLI_MOCK_RESPONSE` | Offline test response |
| `OUR_CLI_MOCK_TOTAL_TOKENS` | Offline test token count |

## License

This project is open source and available under the MIT License.
