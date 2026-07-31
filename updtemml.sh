#!/usr/bin/env bash
set -euo pipefail

JS_URL="https://raw.githubusercontent.com/ronkok/Temml/main/dist/temml.min.js"
CSS_URL="https://raw.githubusercontent.com/ronkok/Temml/main/dist/Temml-Local.css"
CHANGELOG_URL="https://raw.githubusercontent.com/ronkok/Temml/main/CHANGELOG.md"

old_version=""
if [[ -f temml.min.js ]]; then
    old_version=$(grep -oP '\d+\.\d+\.\d+' temml.min.js | head -1)
fi

curl -sLO "$JS_URL"
curl -sLO "$CSS_URL"

new_version=$(grep -oP '\d+\.\d+\.\d+' temml.min.js | head -1)

if [[ -n "$old_version" && "$old_version" == "$new_version" ]]; then
    echo "temml is already up to date (v$new_version)"
    exit 0
fi

echo "temml updated: ${old_version:-unknown} -> $new_version"
echo
echo "latest changelog entry:"
curl -sL "$CHANGELOG_URL" | awk '/^## \[/{n++} n==1{print} n==2{exit}'
