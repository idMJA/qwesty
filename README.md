# Qwesty

A Rust-based Discord Quest Notifier that automatically fetches available quests from Discord's API and sends notifications via Discord Webhooks.

> [!WARNING]  
> This app uses your Discord user token, which technically violates Discord's Terms of Service. Use at your own risk.
> 
> - Your account could be suspended or banned
> - Keep your token private and secure
> - Do not share your token with anyone
> - Use responsibly with reasonable check intervals

## How It Works

1. **Authentication**: Uses your Discord user token to access the Quest API
2. **Quest Fetching**: Polls Discord's `/api/v10/quests/@me` endpoint at configured intervals
3. **Storage**: Tracks previously seen quests in a JSON file (or memory)
4. **Detection**: Compares fetched quests with stored ones to find new quests
5. **Notification**: Sends Discord webhook embeds for new quests only
6. **Multi-locale**: Optionally checks all 33+ Discord locales to find region-specific quests

## Quick Setup

1. **Prerequisites:** Rust 1.70+ ([Install](https://rustup.rs/))
2. **Setup config:**
   ```bash
   cp example.config.toml config.toml
   ```
3. **Edit config:**
   - Add Discord user token ([How to get your token](https://gist.github.com/MarvNC/e601f3603df22f36ebd3102c501116c6))
   - Add webhook URLs under `[[discord.webhooks]]`

> [!TIP]
> You can add multiple webhooks to send notifications to different Discord channels.

4. **Run:**
   ```bash
   cargo run
   ```

## Features

- Fetches quests from Discord API (`https://discord.com/api/v10/quests/@me`)
- Multi-webhook support with optional names
- Reward filtering: `all`, `orbs`, or `decor`
- Persistent quest storage (JSON) or in-memory
- Configurable check intervals
- Multi-locale support (single or all 33+ locales)
- Docker-ready

## Configuration

### Required Fields
```toml
[discord]
token = "your_discord_user_token"

[[discord.webhooks]]
url = "https://discordapp.com/api/webhooks/YOUR_ID/YOUR_TOKEN"
name = "Optional webhook name"  # optional
```

> [!NOTE]
> The `super_properties` field is optional. You can generate it using the browser console if needed.
>
> <details>
> <summary><b>How to generate super_properties</b></summary>
>
> 1. Open Discord in your browser
> 2. Press `F12` to open Developer Tools
> 3. Go to the **Console** tab
> 4. Paste this code and press Enter:
>
> ```javascript
> (() => {
>     const info = {
>         os: navigator.platform || "",
>         browser: navigator.userAgent.includes("Chrome") ? "Chrome" :
>                  navigator.userAgent.includes("Firefox") ? "Firefox" :
>                  navigator.userAgent.includes("Safari") ? "Safari" :
>                  "Unknown",
>         device: "",
>         system_locale: navigator.language || "",
>         has_client_mods: false,
>         browser_user_agent: navigator.userAgent,
>         browser_version: (navigator.userAgent.match(/(Chrome|Firefox|Safari)\/([\d.]+)/) || [undefined,'',''])[2],
>         os_version: navigator.userAgent.match(/(Windows NT|Mac OS X|Android|CPU iPhone OS) ([\d_]+)/)?.[2]?.replace(/_/g,".") || "",
>         referrer: document.referrer || "",
>         referring_domain: document.referrer ? (new URL(document.referrer)).hostname : "",
>         referrer_current: location.href,
>         referring_domain_current: location.hostname,
>         release_channel: "stable",
>         client_build_number: Math.floor(Math.random()*900000 + 100000),
>         client_event_source: null
>     };
>     const json = JSON.stringify(info);
>     const encoded = btoa(json);
>     console.log(encoded);
> })();
> ```
>
> 5. Copy the base64 string that appears in the console
> 6. Add it to your `config.toml` as `super_properties = "your_base64_string"`
>
> </details>

### Optional Fields
| Field | Default | Description |
|-------|---------|-------------|
| `reward_filter` | `all` | Quest reward type: `all`, `orbs`, or `decor` |
| `fetch_interval_minutes` | `30` | Check interval in minutes |
| `locale_mode` | `single` | `single` or `all` (33+ locales) |
| `run_once` | `false` | Exit after first check (useful for cron) |
| `storage_type` | `json` | `json` or `memory` |
| `storage_path` | `./known-quests.json` | Where to store quest data |

## Usage

### Development
```bash
RUST_LOG=info cargo run
```

### Production Build
```bash
cargo build --release
./target/release/qwesty
```

### Docker
```bash
docker build -t qwesty .
docker run -v $(pwd)/config.toml:/app/config.toml:ro -v /data:/data qwesty
```

## Multi-Region Setup (Agent/Collector)

Qwesty supports distributed multi-region quest monitoring through an **Agent/Collector** architecture. This allows you to monitor quests from multiple geographic regions (e.g., Korea, US, Europe) while centralizing notifications.

### Architecture

- **Collector** (Host A): Central hub that receives quest data from agents and sends notifications
  - Runs ingest server on port 8080 (configurable)
  - Also acts as agent for its local region
  - Sends unified notifications via configured webhooks
  
- **Agent** (Hosts B, C, ...): Lightweight workers in different regions
  - Fetches quests for assigned region
  - Sends quest data to Collector
  - No webhooks required

### Collector Setup

```toml
[mode]
role = "collector"
accept_token = "your-shared-secret"  # Agents must use this token
ingest_port = 8080  # Optional, defaults to 8080

[region]
code = "id-ID"  # Your collector's local region

[discord]
token = "your_discord_token"

[[discord.webhooks]]
name = "Global Notifications"
url = "https://discordapp.com/api/webhooks/ID/TOKEN"
```

**Run Collector:**
```bash
cargo run
# or with Docker, expose port 8080
docker run -p 8080:8080 -v $(pwd)/config.toml:/app/config.toml:ro qwesty
```

> [!IMPORTANT]
> Put HTTPS reverse proxy (Caddy/Nginx) in front of the collector for secure communication.

### Agent Setup

```toml
[mode]
role = "agent"
collector_url = "https://your-collector.example.com/ingest"
collector_token = "your-shared-secret"  # Must match collector's accept_token

[region]
code = "ko-KR"  # Agent's region (ko-KR, en-US, ja-JP, etc.)

[discord]
token = "your_discord_token"  # Still required to fetch quests

# No webhooks needed for agents
```

**Run Agent:**
```bash
cargo run
# or
docker run -v $(pwd)/config.toml:/app/config.toml:ro qwesty
```

### API Endpoints

**Collector exposes:**
- `POST /ingest` - Receives quest payloads from agents
  - Auth: `Authorization: Bearer <token>`
  - Body: `{ "region": "ko-KR", "quests": [...], "source": "agent" }`
  - Returns: `{ "accepted": N, "deduped": M }`
- `GET /health` - Health check endpoint

### How It Works

1. **Agent** fetches quests for its configured region
2. **Agent** POSTs quest data to Collector's `/ingest` endpoint with bearer token
3. **Collector** validates token, deduplicates quests by `(region, id)`
4. **Collector** sends Discord notifications for new quests
5. **Collector** persists deduplicated quests to storage

### Security

- Use HTTPS with reverse proxy (Caddy/Nginx recommended)
- Keep `accept_token`/`collector_token` secret and complex
- Rotate tokens periodically
- Optional: IP allowlist for known agents

## Multi-Locale Mode

When `locale_mode = "all"`:
- Checks all 33+ Discord locales sequentially
- 60-70s random delay between checks (rate limiting)
- ~35-40 minutes per full cycle
- Quests deduplicated by ID across locales

## Webhook Notification Format

Notifications are sent as Discord embeds with:
- Quest name and game title
- Reward type (Orbs 🟣, Decorations 🟢, or Other ⚫)
- Expiration time
- Color-coded by reward type

## Troubleshooting

| Issue | Solution |
|-------|----------|
| `config.toml not found` | Run `cp example.config.toml config.toml` |
| No token configured | Add `token = "..."` in `[discord]` section |
| No webhooks configured | Add `[[discord.webhooks]]` with `url` field |
| Failed to fetch quests | Verify token validity with Discord API |
| No notifications sent | Check if webhook URLs are correct and reward filter matches |

## Project Structure

```
src/
├── main.rs             # Entry point, main loop
├── lib.rs              # Library exports
├── models/
│   ├── config.rs       # Config loading & parsing
│   ├── quest.rs        # Quest data structures
│   ├── errors.rs       # Error types
│   └── mod.rs
├── services/
│   ├── quest_client.rs # Discord API client
│   ├── webhook.rs      # Webhook sender
│   ├── storage.rs      # Quest persistence
│   └── mod.rs
└── utils/
    └── mod.rs          # Utilities
```

## API Reference

- **Endpoint:** `GET https://discord.com/api/v10/quests/@me`
- **Auth:** Discord user token in Authorization header
- **Returns:** Array of quest objects with metadata

## License

This project is provided as-is for research and educational purposes only.

This project is licensed under the [**GNU Affero General Public License v3.0**](LICENSE).

