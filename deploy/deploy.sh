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

# 3. Install the supervisord service (Uberspace uses supervisord, NOT
#    systemd user services — `systemctl --user` / linger don't exist there)
echo "==> Installing supervisord service..."
REAL_HOME="$(getent passwd "$USER_NAME" | cut -d: -f6)"
[ -n "$REAL_HOME" ] || REAL_HOME="$HOME"
mkdir -p "$REAL_HOME/etc/services.d"
cat > "$REAL_HOME/etc/services.d/$SERVICE_NAME.ini" <<EOF
[program:$SERVICE_NAME]
command=$APP_DIR/target/release/alice-website
directory=$APP_DIR
startsecs=60
autorestart=true
redirect_stderr=true
EOF
supervisorctl reread
supervisorctl update
supervisorctl restart "$SERVICE_NAME" || supervisorctl start "$SERVICE_NAME"
supervisorctl status "$SERVICE_NAME"

# 4. Point the domain at the local port (reverse proxy)
echo "==> Setting web backend to port $PORT..."
uberspace web backend set / --http --port "$PORT"

echo ""
echo "==> Done! Test with: curl -I https://$(hostname)"
echo "    Logs: supervisorctl tail $SERVICE_NAME"
