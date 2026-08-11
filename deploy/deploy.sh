set -euo pipefail

APP_DIR="$HOME/alicemow"
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
mkdir -p "$HOME/.config/systemd/user"
cp "$APP_DIR/deploy/alice-website.service" "$HOME/.config/systemd/user/$SERVICE_NAME.service"
systemctl --user daemon-reload
systemctl --user enable --now "$SERVICE_NAME"
systemctl --user --no-pager status "$SERVICE_NAME"

# 4. Point the domain at the local port (reverse proxy)
echo "==> Setting web backend to port $PORT..."
uberspace web backend set / --http --port "$PORT"

echo ""
echo "==> Done! Test with: curl -I https://$(hostname)"
echo "    Logs: systemctl --user status $SERVICE_NAME -l"
