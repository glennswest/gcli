# gcli

Grok-powered CLI tool with voice STT, embedded PPTX generation, multi-agent workflows, and auto-git.

## Features

- **Multi-Agent Chat** — Plan + execute via Grok (xAI) with local Ollama fallback
- **Voice Input (STT)** — Press-to-talk microphone recording with Whisper transcription
- **Voice Output (TTS)** — Spoken responses via system TTS
- **PowerPoint Generation** — AI-generated slide decks with embedded Grok images
- **Code Audit** — Secret scanning + LLM-powered code review
- **Web Search** — DuckDuckGo instant answers from the terminal
- **Auto-Git** — AI-generated commit messages from staged changes
- **Self-Update** — GitHub release-based binary updates
- **Project Tracking** — Automatic working directory tracking across sessions

## Requirements

- Rust 1.75+
- `XAI_API_KEY` environment variable (get one at [x.ai](https://x.ai))
- For voice: a working microphone and Whisper model at `~/.gcli/models/whisper-medium-q4_1.bin`
- For local mode: [Ollama](https://ollama.ai) running with `llama3.2`

## Install

Download a pre-built binary from [Releases](https://github.com/glennswest/gcli/releases):

| Platform | Binary |
|----------|--------|
| macOS ARM64 | `gcli-macos-arm64` |
| Linux x86_64 | `gcli-linux-x86_64` |

```bash
# Example: install on Linux
curl -Lo gcli https://github.com/glennswest/gcli/releases/latest/download/gcli-linux-x86_64
chmod +x gcli
sudo mv gcli /usr/local/bin/
```

Or update from an existing install:

```bash
gcli update
```

## Build from Source

```bash
cargo build --release
```

### Multi-platform release build

Builds Linux x86_64 on server1 via podman and macOS ARM64 locally:

```bash
./scripts/build.sh
```

### Cut a release

Bumps version, builds all platforms, tags, and creates a GitHub release:

```bash
./scripts/release.sh patch   # or minor, major
```

## Usage

```bash
# Interactive mode
gcli interactive

# Single chat prompt
gcli chat "explain quicksort"

# With voice input/output
gcli interactive --voice --voice-input

# Generate a PowerPoint
gcli ppt "Q3 Report" "quarterly business results for SaaS company"

# Audit a file or directory for secrets + code issues
gcli audit src/main.rs

# Web search
gcli search "rust async patterns"

# AI-assisted git commit
gcli git commit

# Self-update from GitHub releases
gcli update

# List tracked projects
gcli projects

# Configure API key
gcli configure

# Test voice STT
gcli voice-test
```

### Flags

| Flag | Description |
|------|-------------|
| `--local` | Use local Ollama instead of xAI API |
| `--voice` | Enable TTS output |
| `--voice-input` | Enable STT input (press-to-talk) |

## License

MIT
