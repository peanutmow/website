set -euo pipefail

# Canonical Uberspace web root. In some shells $HOME resolves to /home/<user>
# while the site actually lives under /var/www/virtual/<user>/html.
USER_NAME="$(whoami)"
if [ -d "/var/www/virtual/$USER_NAME/html" ]; then
  APP_DIR="/var/www/virtual/$USER_NAME/html"
else
  APP_DIR="$HOME/html"
fi
SERVICE_NAME="alice-website"
PORT=8080

# 1. Make sure Rust is available
if ! command -v cargo >/dev/null 2>&1; then
  echo "==> Installing Rust (rustup)..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
fi

# 2. Build the release binary
echo "==> Building release binary..."
cd "$APP_DIR"
cargo build --release

# 3. Install the systemd user service
echo "==> Installing systemd user service..."
REAL_HOME="$(getent passwd "$USER_NAME" | cut -d: -f6)"
[ -n "$REAL_HOME" ] || REAL_HOME="$HOME"

# Make sure systemctl --user can talk to the session bus
export XDG_RUNTIME_DIR="/run/user/$(id -u)"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
loginctl enable-linger "$USER_NAME" 2>/dev/null || true

mkdir -p "$REAL_HOME/.config/systemd/user"
cat > "$REAL_HOME/.config/systemd/user/$SERVICE_NAME.service" <<EOF
[Unit]
Description=Alice Website (Rust/axum SSR)
After=network.target

[Service]
WorkingDirectory=$APP_DIR
ExecStart=$APP_DIR/target/release/alice-website
Restart=always
RestartSec=2
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
EOF
systemctl --user daemon-reload
systemctl --user enable --now "$SERVICE_NAME"
systemctl --user --no-pager status "$SERVICE_NAME"

# 4. Point the domain at the local port (reverse proxy)
echo "==> Setting web backend to port $PORT..."
uberspace web backend set / --http --port "$PORT"

echo ""
echo "==> Done! Test with: curl -I https://$(hostname)"
echo "    Logs: systemctl --user status $SERVICE_NAME -l"
