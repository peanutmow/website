#!/usr/bin/env bash
set -euo pipefail

DOMAIN="${DOMAIN:-blog.alicemow.org}"
PORT="${PORT:-8082}"
MAIN="${MAIN:-https://alicemow.org}"
LANDING="${LANDING:-/alice}"

USER_NAME="$(id -un)"
HOME_DIR="$(getent passwd "$USER_NAME" | cut -d: -f6)"
[ -n "$HOME_DIR" ] || HOME_DIR="$HOME"
DOCROOT="/var/www/virtual/$USER_NAME/$DOMAIN"

printf '==> writing SEO files to %s\n' "$DOCROOT"
mkdir -p "$DOCROOT"

python3 - "$DOMAIN" "$PORT" "$MAIN" "$DOCROOT" <<'PY'
import sys
import urllib.request
import xml.etree.ElementTree as ET
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
from pathlib import Path

domain, port, main, docroot = sys.argv[1:5]
root = Path(docroot)
sections = [
    ("alice", "Main", "technology and personal writing"),
    ("politics", "Politics", "political writing"),
]

def local(tag):
    return tag.rsplit("}", 1)[-1]

def w3c(value):
    if not value:
        return None
    try:
        parsed = parsedate_to_datetime(value)
    except (TypeError, ValueError):
        try:
            parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
        except ValueError:
            return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

def escape(value):
    return value.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")

urls = {}
for slug, label, description in sections:
    page = f"https://{domain}/{slug}/"
    urls[page] = None
    feed = f"http://127.0.0.1:{port}/{slug}/feed/"
    try:
        data = urllib.request.urlopen(feed, timeout=5).read()
        tree = ET.fromstring(data)
    except Exception as error:
        print(f"warning: could not read {slug} feed: {error}", file=sys.stderr)
        continue
    for node in tree.iter():
        if local(node.tag) not in ("item", "entry"):
            continue
        link = ""
        date = None
        for child in node:
            name = local(child.tag)
            if name == "link":
                if child.text and child.text.strip().startswith("http"):
                    link = child.text.strip()
                elif child.get("rel") in (None, "alternate") and child.get("href"):
                    link = child.get("href")
            elif name in ("pubDate", "published", "updated"):
                date = child.text
        if link:
            urls[link] = date

lines = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
]
for loc, date in urls.items():
    lastmod = w3c(date)
    if lastmod:
        lines.append(f"  <url><loc>{escape(loc)}</loc><lastmod>{lastmod}</lastmod></url>")
    else:
        lines.append(f"  <url><loc>{escape(loc)}</loc></url>")
lines.append("</urlset>")
(root / "sitemap.xml").write_text("\n".join(lines) + "\n", encoding="utf-8")

(root / "robots.txt").write_text(
    "User-agent: *\n"
    "Content-Signal: search=yes, ai-input=yes, ai-train=yes\n"
    "Allow: /\n"
    "\n"
    f"Sitemap: https://{domain}/sitemap.xml\n",
    encoding="utf-8",
)

blog_lines = [
    "# Alice's blog",
    "",
    "> Personal and political writing by Alice (alicemow).",
    "",
    "## Main",
]
for slug, label, description in sections:
    blog_lines.append(f"- [{label}](https://{domain}/{slug}/): {description}.")
blog_lines += ["", "## Feeds"]
for slug, _, _ in sections:
    blog_lines.append(f"- https://{domain}/{slug}/feed/")
blog_lines += [
    "",
    "## Site",
    f"- [alicemow.org]({main}): the portfolio, projects and gallery.",
]
(root / "llms.txt").write_text("\n".join(blog_lines) + "\n", encoding="utf-8")
PY

chmod 755 "$DOCROOT"
chmod 644 "$DOCROOT/robots.txt" "$DOCROOT/sitemap.xml" "$DOCROOT/llms.txt"
restorecon -R "$DOCROOT" 2>/dev/null || true

command -v uberspace >/dev/null 2>&1 || export PATH="$PATH:/usr/local/bin"

printf '==> routing discovery files through Apache\n'
for file in robots.txt sitemap.xml llms.txt; do
    uberspace web backend set "$DOMAIN/$file" --apache
done

CONFIG="$HOME_DIR/writefreely/config.ini"
if [ -f "$CONFIG" ] && grep -Eq '^landing[[:space:]]*=' "$CONFIG"; then
    if ! grep -Eq "^landing[[:space:]]*=[[:space:]]*${LANDING}/?[[:space:]]*$" "$CONFIG"; then
        printf '==> setting blog landing page to %s\n' "$LANDING"
        sed -i -E "s|^landing[[:space:]]*=.*|landing = ${LANDING}|" "$CONFIG"
        supervisorctl restart writefreely || true
    fi
fi

printf '==> done\n'
printf 'robots:  https://%s/robots.txt\n' "$DOMAIN"
printf 'sitemap: https://%s/sitemap.xml\n' "$DOMAIN"
printf 'llms:    https://%s/llms.txt\n' "$DOMAIN"
