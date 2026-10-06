# Tickit

Self-hosted todo app written in Rust. Fast, single binary, SQLite storage, email reminders and TOTP 2FA.

## Features

- Rust + Axum backend, SQLite database in `/var/lib/tickit/`
- Alpine.js + Tailwind CSS frontend, History API routing
- Desktop + mobile friendly
- Markdown descriptions with editor and preview
- Email reminders:
  - Daily at your configured time (default 07:00, server timezone) with todos due today
  - 15 minutes before a todo's due time (when a time is set)
- Error mails with device info (IP, user-agent, method, path)
- TOTP 2FA via any authenticator app
- Persistent login cookie (7 days)
- Rate limiting on login/register (5 attempts per 5 minutes)
- JSON export (per user)
- English / Nederlands with in-app language switcher

## Requirements

- Rust toolchain (`cargo build --release`)
- An SMTP account (Gmail app-password or Proton) for reminders and error mail
- A small GNU/Linux server

## Build

```bash
cargo build --release
```

The binary is `target/release/tickit`. Static assets are served from the `assets/`
directory relative to the working directory.

## Configuration

Copy `.env.example` to `.env` and fill it in. The real `.env` is gitignored and must
never be committed (the repository is public).

```bash
cp .env.example .env
$EDITOR .env
```

Key fields:

| Variable | Purpose |
|---|---|
| `BIND_ADDR` / `BIND_PORT` | Listener address. Use `127.0.0.1:8081` and reverse-proxy to it |
| `APP_SECRET` | Long random string for session integrity |
| `DATA_DIR` / `DB_PATH` | Where SQLite lives (defaults under `/var/lib/tickit/`) |
| `SMTP_*` / `MAIL_*` | SMTP credentials + recipient for error mails |
| `COOKIE_SECURE` | Set `true` when behind HTTPS |
| `REGISTRATION_OPEN` | See below |
| `RATE_LIMIT_PER_5MIN` | Login/register attempts per 5 minutes |
| `REMINDER_OFFSET_MIN` | Minutes before a due time to email (default 15) |

The first account is created via the "Register" link on the login page. After creating
your account you can disable open registration by setting `REGISTRATION_OPEN=false` in
`.env` and restarting.

## Local run

```bash
./target/release/tickit
```

Open `http://localhost:8081` (or your configured port).

## Deploy to server (SSH)

1. Create a release from GitHub (tag `vX.Y.Z`) - the GitHub Actions workflow builds the
   binary and publishes a tarball.

2. On the server, create the user and directories:

```bash
sudo useradd --system --home-dir /var/lib/tickit --shell /usr/sbin/nologin tickit
sudo mkdir -p /var/lib/tickit /etc/tickit
```

3. Download and extract the release tarball:

```bash
cd /tmp
curl -Lo tickit.tar.gz https://github.com/LieutenantHazzy/Tickit/releases/download/vX.Y.Z/tickit-vX.Y.Z-x86_64-linux.tar.gz
tar xzf tickit.tar.gz
sudo cp tickit/tickit /usr/local/bin/
sudo chmod +x /usr/local/bin/tickit
sudo cp -r tickit/assets /var/lib/tickit/
sudo cp tickit/.env.example /etc/tickit/.env
sudo $EDITOR /etc/tickit/.env   # configure real values
```

4. Install the systemd unit:

```bash
sudo cp tickit/tickit.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now tickit
```

5. Ownership:

```bash
sudo chown -R tickit:tickit /var/lib/tickit
```

6. Reverse proxy (example Caddy):

```
live.example.com {
    reverse_proxy 127.0.0.1:8081
}
```

## Update to a new version

```bash
cd /tmp
curl -Lo tickit.tar.gz https://github.com/LieutenantHazzy/Tickit/releases/download/vX.Y.Z/tickit-vX.Y.Z-x86_64-linux.tar.gz
tar xzf tickit.tar.gz
sudo systemctl stop tickit
sudo cp tickit/tickit /usr/local/bin/
sudo cp -r tickit/assets /var/lib/tickit/
sudo systemctl start tickit
```

## API overview

| Route | Method | Description |
|---|---|---|
| `/api/auth/register` | POST | Create account |
| `/api/auth/login` | POST | Login (optional `totp_code`) |
| `/api/auth/logout` | POST | Logout |
| `/api/auth/me` | GET | Current user + preferences |
| `/api/auth/preferences` | PATCH | Set language + daily reminder time |
| `/api/auth/setup-2fa` | GET/POST | Setup / verify TOTP |
| `/api/auth/disable-2fa` | POST | Disable 2FA |
| `/api/todos` | GET/POST | List / create |
| `/api/todos/:id/toggle` | POST | Toggle done |
| `/api/todos/export/json` | GET | Export all todos + lists |
| `/api/lists` | GET/POST | List / create |
| `/api/healthz` | GET | Health check |

## Security notes

- Passwords hashed with Argon2id
- Sessions are random UUIDs stored server-side, HTTP-only cookies with 7-day lifetime
- Login/register rate limited per IP (5 per 5 minutes)
- TOTP secrets are per-user; QR shown once during setup
- Behind a reverse proxy, set `COOKIE_SECURE=true` and make sure the proxy only trusts
  its own `X-Forwarded-For` header

## License

MIT