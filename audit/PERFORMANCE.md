# Performance probe

Measured against the live local server on this machine (air-gapped, warm cache after first call).

| Route | Median ms |
|---|---:|
| `/status` | 0.6 |
| `/surface` | 1.7 |
| `/atlas` | 1.0 |
| `/atlas/graph?offset=0` | 0.4 |
| `/atlas/schema` | 0.7 |
| `/atlas/health` | 0.8 |

Server caps: 5000 nodes / 10000 edges. Client renders the full SVG; table fallback exists for oversized graphs.
No progress percentage is synthesised; elapsed timers are real.
