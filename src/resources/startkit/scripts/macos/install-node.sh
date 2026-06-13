#!/usr/bin/env sh
set -eu

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

mkdir -p "$STARTKIT_CACHE_DIR"

index_file="$STARTKIT_CACHE_DIR/node-index.json"
curl -fsSL "${STARTKIT_NODE_INDEX_URL:-https://nodejs.org/dist/index.json}" -o "$index_file"

min_major="$(printf '%s' "${STARTKIT_MIN_VERSION:-22.0.0}" | sed 's/^v//' | cut -d. -f1)"
node_version="$(
  tr '{' '\n' < "$index_file" | awk -F'"' -v min_major="$min_major" '
    /"version"/ && /"lts":/ && !/"lts":false/ {
      for (i = 1; i <= NF; i++) {
        if ($i == "version") {
          version = $(i + 2)
          major = version
          sub(/^v/, "", major)
          sub(/\..*/, "", major)
          if ((major + 0) >= (min_major + 0)) {
            print version
            exit
          }
        }
      }
    }
  '
)"

if [ -z "$node_version" ]; then
  printf '{"status":"error","message":"Could not resolve a Node.js LTS version","actions":["install"]}\n'
  exit 0
fi

pkg_name="node-${node_version}.pkg"
download_url="${STARTKIT_NODE_DIST_BASE:-https://nodejs.org/dist}/${node_version}/${pkg_name}"
pkg_path="$STARTKIT_CACHE_DIR/$pkg_name"

curl -fL "$download_url" -o "$pkg_path"
open "$pkg_path"

printf '{"status":"blocked","message":"Node.js installer was opened. Complete it, then run setup again.","path":"%s","actions":["verify"]}\n' \
  "$(json_escape "$pkg_path")"
