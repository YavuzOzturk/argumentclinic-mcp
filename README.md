# ArgumentClinic MCP

Adversarial reasoning layer for decisions that matter — CLI tool and MCP server.

Full product at [argumentclinic.io](https://argumentclinic.io).

---

## What It Does

ArgumentClinic runs a claim through a four-stage adversarial pipeline before returning an answer:

1. **Attack** — an adversarial model finds the strongest possible objection to the claim
2. **Defense** — a second model argues why the claim survives despite the attack
3. **Judge** — a third model evaluates whether the defense successfully rebutted the attack, returning a structured verdict with a confidence score
4. **Synthesize** — a final model produces a refined version of the claim incorporating what survived the debate

Each stage uses a separate LLM call. You can assign different models to different roles, or use the same model for all four.

### PAUSED behavior

When the judge determines that the claim did not survive with high confidence (confidence > 0.7, survived = false), the synthesizer returns `PAUSED` instead of an answer. This is a deliberate refusal — the system will not produce a confident output when the evidence is insufficient. The PAUSED response includes the reason from the judge.

### Provider support

Works with any LLM provider: OpenAI, Anthropic, DeepSeek, Gemini, Grok, Ollama, or any OpenAI-compatible endpoint. Each pipeline role can use a different provider and model.

---

## Installation

### Download Binary

Prebuilt binaries are available on the [GitHub releases page](https://github.com/YavuzOzturk/argumentclinic-mcp/releases).

**Linux:**

```sh
curl -L https://github.com/YavuzOzturk/argumentclinic-mcp/releases/latest/download/argumentclinic-linux-x86_64 \
  -o argumentclinic
chmod +x argumentclinic
mv argumentclinic ~/.local/bin/
```

**Windows:**

Download `argumentclinic-windows-x86_64.exe` from the releases page and add its location to your `PATH`.

### Build From Source

Requirements: Rust 1.75 or later.

```sh
git clone https://github.com/YavuzOzturk/argumentclinic-mcp.git 
cd argumentclinic-mcp
cargo build --release
```

Binary is at `target/release/argumentclinic`.

---

## Configuration

### Quick Start

```sh
# Configure a provider
argumentclinic config set-provider openai --api-key sk-...

# Assign models to each pipeline role
argumentclinic config set-role attacker    openai gpt-4o-mini
argumentclinic config set-role defender   openai gpt-4o-mini
argumentclinic config set-role judge      openai gpt-4o-mini
argumentclinic config set-role synthesizer openai gpt-4o-mini

# Run your first analysis
argumentclinic analyze --text "your claim here"
```

### Config File Location

| Platform | Path |
|----------|------|
| Linux    | `~/.config/argumentclinic/config.yaml` |
| Windows  | `%APPDATA%\argumentclinic\config.yaml` |

See `config.example.yaml` in this repository for the full reference including connector configuration.

### Supported Providers

| Provider | Key Name | Notes |
|----------|----------|-------|
| OpenAI | `openai` | |
| Anthropic | `anthropic` | |
| DeepSeek | `deepseek` | |
| Gemini | `gemini` | API key passed as query parameter |
| Grok | `grok` | |
| Ollama | any name | Use connector config; no auth required |
| Any OpenAI-compatible | any name | Use connector config with `base_url` |

### Custom Connector

For local models or non-standard endpoints, use the connector configuration. Ollama example:

```yaml
providers:
  ollama-local:
    connector:
      url: "http://localhost:11434/api/generate"
      auth:
        type: none
      format: ollama
```

Then assign roles to `ollama-local` as normal:

```sh
argumentclinic config set-role attacker ollama-local llama3.2
```

---

## Usage

### CLI

```sh
# Analyze a claim passed as a string
argumentclinic analyze --text "Remote work improves productivity for software engineers."

# Analyze the contents of a file
argumentclinic analyze --file proposal.md

# Pipe from stdin
echo "This architecture will scale to 10M users." | argumentclinic analyze

# View current configuration (API keys are redacted)
argumentclinic config show

# Full help
argumentclinic --help
argumentclinic analyze --help
argumentclinic config --help
```

### MCP Server (Cursor)

Add to your Cursor `mcp.json` (typically `~/.cursor/mcp.json`):

```json
{
  "mcpServers": {
    "argumentclinic": {
      "command": "/path/to/argumentclinic",
      "args": ["serve"]
    }
  }
}
```

Replace `/path/to/argumentclinic` with the absolute path to the binary. Restart Cursor after saving.

### MCP Server (Claude Desktop)

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "argumentclinic": {
      "command": "/path/to/argumentclinic",
      "args": ["serve"]
    }
  }
}
```

On Linux the config file is at `~/.config/claude/claude_desktop_config.json`. On macOS it is at `~/Library/Application Support/Claude/claude_desktop_config.json`. Restart Claude Desktop after saving.

The MCP server exposes a single tool, `analyze`, which accepts a `content` string (required) and an optional `context` string.

---

## Output Format

```
=== ArgumentClinic Analysis ===

CLAIM:
Remote work improves productivity for software engineers.

ATTACK:
The claim lacks specificity about what "productivity" means and for
which types of tasks. Studies on remote work show mixed results
depending on role seniority, home environment, and collaboration
requirements. A stronger claim would specify the conditions under
which the effect holds.

DEFENSE:
Multiple large-scale studies (Stanford, Microsoft Research) show
output increases of 13-20% for individual contributor roles, which
constitute the majority of software engineering positions. The claim
holds for this population even if it does not generalize universally.

JUDGMENT: survived (confidence: 0.78)
The defense provided specific empirical evidence that the attack's
demand for specificity does not invalidate the core claim for the
primary population.

VERDICT: SUPPORTED — claim survived adversarial pressure
Remote work demonstrably improves productivity for individual
contributor software engineers based on available empirical evidence,
though the effect varies by role type and working conditions.
```

---

## Pro Mode

Pro mode connects to the [argumentclinic.io](https://argumentclinic.io) API, which uses benchmark data to automatically select the optimal model for each pipeline role based on the task type. You configure your API key and providers once; the API handles model routing.

```sh
argumentclinic config set-ag-key your-key-here
argumentclinic config set-mode pro
```

In pro mode, the `roles` section of your config is ignored. Model assignments come from the API response based on your query.

---

## License

MIT
