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
# 第六/七/八件（可选）：交互契约 + 直觉判定 + 认知走查；UI 变更必填，其余变更可保留为 planned 或删除。
# 仅在本脚本新建的 change 目录内生成；模板不存在时保持旧行为（只生成五件套）。
for f in interaction intuition intuition-walkthrough; do
  src="$root/specs/templates/$f.md"
  if [ -f "$src" ] && [ ! -e "$dir/$f.md" ]; then
    cp "$src" "$dir/$f.md"
    sed -i "s/<change-id>/$id/g" "$dir/$f.md"
  fi
done
printf 'created %s\n' "${dir#$root/}"
