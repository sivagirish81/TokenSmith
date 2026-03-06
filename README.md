# tokensmith

Stop Paying the Token Tax.

`tokensmith` is a Rust CLI that detects your hardware, recommends a local model/runtime configuration, manages model downloads, starts local serving, and exposes OpenAI-compatible APIs. It focuses on safe defaults, OOM avoidance, and operator controls for monitoring and safe stop.

## What is tokensmith?

Running local LLMs is usually manual and brittle: model choice is unclear, memory limits are easy to exceed, and runtime operations are fragmented. `tokensmith` orchestrates this workflow:

- Hardware profiling (`doctor`)
- Explainable model/config recommendation (`recommend`)
- Model artifact management (`pull`)
- Runtime/server lifecycle (`up`, `stop`, `ps`, `logs`)
- OpenAI-compatible serving (`/v1/chat/completions`, `/v1/completions`, SSE streaming)
- Resource monitoring with warning thresholds (`status`, `monitor`)

## Quickstart

```bash
tokensmith doctor
tokensmith recommend --task code --mode balanced
tokensmith pull <id>
tokensmith up --task code --mode balanced --detach
tokensmith status
tokensmith monitor --watch
tokensmith stop
```

## Commands

- `tokensmith doctor`
- `tokensmith recommend --task code|chat [--mode fast|balanced|quality]`
- `tokensmith pull <model_id>`
- `tokensmith up --task code|chat [--mode fast|balanced|quality] [--ctx 4096] [--port 8000] [--host 127.0.0.1] [--detach]`
- `tokensmith status`
- `tokensmith monitor [--interval 1s] [--watch] [--json] [--warn-mem 80%] [--warn-cpu 300%]`
- `tokensmith throttle --mode fast|balanced|quality`
- `tokensmith stop [--force-after 5s]`
- `tokensmith kill`
- `tokensmith ps`
- `tokensmith logs [--follow]`

## Monitoring and Safe Stop

Use `tokensmith monitor` to inspect:

- RSS memory (MB)
- CPU %
- thread count
- uptime
- system total/free memory when available

Threshold warnings:

```bash
tokensmith monitor --watch --warn-mem 80% --warn-cpu 300%
```

When warnings trigger, use:

```bash
tokensmith throttle --mode fast
tokensmith stop
```

Safe stop behavior:

1. Send SIGTERM
2. Wait `--force-after` (default `5s`)
3. Escalate to SIGKILL if still alive

## OpenAI-Compatible Clients

```bash
export OPENAI_BASE_URL=http://127.0.0.1:8000/v1
export OPENAI_API_KEY=local
```

Then use your normal OpenAI SDK pointing at `OPENAI_BASE_URL`.

## Continue Extension Integration (VS Code)

Use a dedicated VS Code profile/workspace so this does not affect your main cloud OpenAI setup.

1. Start tokensmith:

```bash
tokensmith up --task code --mode fast --ctx 4096 --port 8000 --host 127.0.0.1 --detach
```

2. Point Continue to tokensmith (OpenAI-compatible endpoint):

- `apiBase`: `http://127.0.0.1:8000/v1`
- `apiKey`: `local`
- `model`: your loaded local model id (for example `qwen2.5-3b-instruct`)

Example Continue config snippet:

```yaml
models:
  - title: Local Qwen (tokensmith)
    provider: openai
    model: qwen2.5-3b-instruct
    apiBase: http://127.0.0.1:8000/v1
    apiKey: local
```

3. Validate quickly:

```bash
tokensmith logs --calls --follow
```

You should see `model_call proxied` / `model_call proxied_stream` lines during Continue requests.

### Continue Troubleshooting

- `502 ... exceed_context_size_error`:
  Request context (chat history + attached files + instructions) is larger than runtime `n_ctx`.
  Fix by lowering context in Continue, starting a new chat, or increasing startup context with `--ctx`.
- `502 ... 503 Loading model`:
  Runtime is still loading. Wait and retry.
- `502 ... error sending request for url (http://127.0.0.1:8001...)`:
  `llama-server` backend is not reachable. Check startup logs and runtime process.
- `llama-server exited early ... model is corrupted or incomplete`:
  Re-pull model artifact.

## Notes

- macOS Metal path (MVP) is designed for `llama.cpp` (`llama-server`) first.
- Binary search order:
  1. `~/.tokensmith/bin/llama-server`
  2. `PATH`
- If `llama-server` is missing, `doctor` provides guidance and the adapter can still respond with placeholder behavior for local testing.

## Roadmap

- CUDA backend support
- Better Windows process/metrics parity
- LAN sharing mode
- Larger curated model registry
- Multi-process management (`tokensmith ps` for more than one active server)

## Development

```bash
cargo fmt
cargo test
cargo run -- recommend --task code --mode balanced
```
