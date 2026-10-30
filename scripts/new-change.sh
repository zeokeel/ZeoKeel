#!/usr/bin/env bash
set -euo pipefail
slug="${1:?用法: bash scripts/new-change.sh <kebab-slug>}"
case "$slug" in
  *[!a-z0-9-]*) echo "slug 只能包含小写字母/数字/连字符: $slug" >&2; exit 2 ;;
esac
root="$(cd "$(dirname "$0")/.." && pwd)"
id="$(date +%Y-%m-%d)-${slug}"
dir="$root/specs/changes/$id"
if [ -e "$dir" ]; then
  echo "已存在: $dir" >&2
  exit 1
fi
mkdir -p "$dir"
for f in proposal spec design tasks acceptance; do
  cp "$root/specs/templates/$f.md" "$dir/$f.md"
  sed -i "s/<change-id>/$id/g" "$dir/$f.md"
done
printf 'created %s\n' "${dir#$root/}"
