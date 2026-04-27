# our-cli

`our-cli` is a Rust twin of `asm-agent`: a macOS-friendly CLI that talks to OpenAI, remembers local conversation history, prints token usage, and supports configurable terminal colors.

## Requirements

- macOS
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

## Run

```sh
export OPENAI_API_KEY="your_api_key"
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

## Local Install

```sh
cargo install --path .
our-cli "hello"
```

After publishing the repo to GitHub, another Mac with Rust installed can install it directly:

```sh
cargo install --git https://github.com/CherifD/our-cli.git
our-cli "hello from GitHub"
```

## Memory

Conversation memory is saved as plain text:

```text
~/.config/our-cli/conversation.txt
```

Use:

```sh
our-cli history
our-cli reset
```

## Colors

Defaults:

```sh
OUR_CLI_PROMPT_COLOR=b84367fc
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
