#!/usr/bin/env bash
#
# Install or update WriteFreely on Uberspace (user-level, no root).
#
#   ./setup.sh                            # fresh install, prompts for the admin password
#   ADMIN_PASS='...' ./setup.sh           # non-interactive
#
# Safe to re-run. An existing config.ini, the keys/ directory and the database
# are never overwritten, so running it again just swaps in the new binary and
# applies any database migrations -- that is also the upgrade path (bump
# VERSION and SHA256 to the release you want).
#
# Run it on the Uberspace host as your own user, from this directory.
#
# Why a subdomain and not alicemow.org/blog: federation needs the instance at
# the root of a host. WriteFreely strips any path out of [app] host, and its
# /.well-known/ and /api/collections/ routes are root-relative, so a subpath
# silently breaks ActivityPub.

set -euo pipefail

DOMAIN="${DOMAIN:-blog.alicemow.org}"
PORT="${PORT:-8082}"
VERSION="${VERSION:-0.17.2}"
ARCH="${ARCH:-linux_amd64}"
SHA256="${SHA256:-332efcf09236a8ca5ffd91379c00d96a6f59ac52812d792cfcf6ee49a67dd3d9}"
SERVICE_NAME="${SERVICE_NAME:-writefreely}"
ADMIN_USER="${ADMIN_USER:-alice}"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
USER_NAME="$(id -un)"
# Uberspace keeps the real home in passwd; $HOME can point into the web space.
HOME_DIR="$(getent passwd "$USER_NAME" | cut -d: -f6)"
[ -n "$HOME_DIR" ] || HOME_DIR="$HOME"
APP_DIR="$HOME_DIR/$SERVICE_NAME"
TARBALL="writefreely_${VERSION}_${ARCH}.tar.gz"
URL="https://github.com/writefreely/writefreely/releases/download/v${VERSION}/${TARBALL}"

log() { printf '==> %s\n' "$*"; }
die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[ "$(uname -s)" = "Linux" ] || die "this script targets Linux; run it on Uberspace"
command -v curl >/dev/null || die "curl not found"

log "installing WriteFreely $VERSION into $APP_DIR"
mkdir -p "$APP_DIR"

# --- 1. download and verify -------------------------------------------------
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

log "downloading $TARBALL"
curl -fsSL -o "$TMP_DIR/$TARBALL" "$URL" || die "download failed"

if command -v sha256sum >/dev/null; then
    printf '%s  %s\n' "$SHA256" "$TMP_DIR/$TARBALL" | sha256sum -c - >/dev/null \
        || die "checksum mismatch -- expected $SHA256"
    log "checksum OK"
fi

tar -xzf "$TMP_DIR/$TARBALL" -C "$TMP_DIR"
SRC="$TMP_DIR/$SERVICE_NAME"
[ -d "$SRC" ] || die "unexpected archive layout in $TARBALL"

# --- 2. install files -------------------------------------------------------
# Everything except keys/ is replaced. keys/ is the instance identity: lose it
# and every existing follower breaks, so it is never touched after first run.
log "copying files (leaving keys/ alone)"
for item in "$SRC"/*; do
    name="$(basename "$item")"
    [ "$name" = "keys" ] && continue
    rm -rf "${APP_DIR:?}/$name"
    cp -r "$item" "$APP_DIR/"
done
chmod +x "$APP_DIR/$SERVICE_NAME"

# --- 3. configuration -------------------------------------------------------
if [ ! -f "$APP_DIR/config.ini" ]; then
    log "writing config.ini for https://$DOMAIN"
    sed -e "s|^port = .*|port = $PORT|" \
        -e "s|^host = .*|host = https://$DOMAIN|" \
        "$HERE/config.ini.example" > "$APP_DIR/config.ini"
    chmod 600 "$APP_DIR/config.ini"
else
    log "keeping existing config.ini"
fi

cd "$APP_DIR"

# --- 4. keys and database ---------------------------------------------------
if [ -f keys/csrf.aes256 ]; then
    log "keys already present"
else
    log "generating keys"
    ./"$SERVICE_NAME" keys generate
fi

FRESH=0
if [ -f writefreely.db ]; then
    log "migrating existing database"
    ./"$SERVICE_NAME" db migrate
else
    FRESH=1
    log "initialising database"
    ./"$SERVICE_NAME" db init
fi

if [ "$FRESH" = "1" ]; then
    if [ -z "${ADMIN_PASS:-}" ]; then
        printf 'Choose a password for the %s admin account: ' "$ADMIN_USER"
        read -rs ADMIN_PASS
        printf '\n'
    fi
    [ -n "$ADMIN_PASS" ] || die "empty password"
    log "creating admin account '$ADMIN_USER'"
    ./"$SERVICE_NAME" user create --admin "$ADMIN_USER:$ADMIN_PASS" \
        || log "admin already exists, continuing"
fi

# --- 5. run it under supervisord --------------------------------------------
# Uberspace supervises user daemons with supervisord; systemd user services and
# linger do not exist there.
log "writing supervisord unit"
mkdir -p "$HOME_DIR/etc/services.d"
cat > "$HOME_DIR/etc/services.d/$SERVICE_NAME.ini" <<INI
[program:$SERVICE_NAME]
command=$APP_DIR/$SERVICE_NAME
directory=$APP_DIR
startsecs=10
autorestart=true
redirect_stderr=true
INI

supervisorctl reread
supervisorctl update
supervisorctl restart "$SERVICE_NAME" || supervisorctl start "$SERVICE_NAME"
supervisorctl status "$SERVICE_NAME" || true

# --- 6. reverse proxy -------------------------------------------------------
log "pointing $DOMAIN at port $PORT"
command -v uberspace >/dev/null 2>&1 || export PATH="$PATH:/usr/local/bin"
# Harmless if the domain is already registered.
uberspace web domain add "$DOMAIN" || true
uberspace web backend set "$DOMAIN/" --http --port "$PORT"

cat <<EOF

Done. Federation must be reachable at the domain root -- check all of these
return JSON, not HTML:

  curl -s "https://$DOMAIN/.well-known/webfinger?resource=acct:$ADMIN_USER@$DOMAIN"
  curl -s  https://$DOMAIN/.well-known/nodeinfo
  curl -sH 'Accept: application/activity+json' https://$DOMAIN/api/collections/$ADMIN_USER

Then:
  1. log in at https://$DOMAIN/login
  2. Blogs -> "New blog" -> create the second blog (slug "politics")
  3. confirm both feeds: /$ADMIN_USER/feed/ and /politics/feed/
  4. follow @politics@$DOMAIN from Mastodon to prove federation works

Keep the DNS record for $DOMAIN DNS-only (grey cloud). Proxying it through a
CDN with bot protection would challenge the other servers that fetch the
ActivityPub endpoints, and federation would fail silently.
EOF
