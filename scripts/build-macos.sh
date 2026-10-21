#!/usr/bin/env bash
# 互联 · macOS 一键构建
# 用法：
#   bash scripts/build-macos.sh                 # 构建 dmg + .app
#   bash scripts/build-macos.sh --debug         # 构建 debug 版（透传给 tauri build）
#   source scripts/env.sh && bash scripts/build-macos.sh   # 使用项目内 Rust 工具链
# 产物：
#   src-tauri/target/release/bundle/dmg/互联_<版本>_<arch>.dmg
#   src-tauri/target/release/bundle/macos/互联.app
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# ── 输出样式（非 TTY 时自动退化为纯文本）────────────────────────────
if [ -t 1 ]; then
  C_RESET="$(printf '\033[0m')"
  C_GREEN="$(printf '\033[32m')"
  C_YELLOW="$(printf '\033[33m')"
  C_RED="$(printf '\033[31m')"
  C_DIM="$(printf '\033[2m')"
else
  C_RESET="" C_GREEN="" C_YELLOW="" C_RED="" C_DIM=""
fi

info() { printf '%s==>%s %s\n' "$C_GREEN" "$C_RESET" "$*"; }
tip()  { printf '%s[!]%s %s\n' "$C_YELLOW" "$C_RESET" "$*"; }
ok()   { printf '%s[✓]%s %s\n' "$C_GREEN" "$C_RESET" "$*"; }
die()  { printf '%s[x]%s %s\n' "$C_RED" "$C_RESET" "$*" >&2; exit 1; }

# ── 0. 平台检查 ────────────────────────────────────────────────────
if [ "$(uname -s)" != "Darwin" ]; then
  die "本脚本仅用于 macOS。Windows 请用 scripts/build-windows.ps1；Android 请用 scripts/build-android.sh。"
fi
ARCH="$(uname -m)"
info "目标平台：macOS ${ARCH}（$(sw_vers -productVersion 2>/dev/null || echo '版本未知')）"

# ── 1. Rust（优先使用项目内 .toolchain）───────────────────────────
if ! command -v cargo >/dev/null 2>&1; then
  if [ -f "$ROOT/scripts/env.sh" ]; then
    # shellcheck disable=SC1091
    . "$ROOT/scripts/env.sh"
    info "已加载项目内 Rust 工具链（scripts/env.sh）"
  fi
fi
command -v cargo >/dev/null 2>&1 \
  || die "未找到 cargo。请安装 rustup（https://rustup.rs/），或先执行 source scripts/env.sh 使用项目内工具链。"
command -v rustc >/dev/null 2>&1 || die "未找到 rustc（Rust 安装不完整，重装 rustup 后再试）。"
ok "Rust：$(rustc -V)"

# ── 2. Xcode Command Line Tools ────────────────────────────────────
if ! xcrun --show-sdk-path >/dev/null 2>&1; then
  die "缺少 Xcode Command Line Tools。执行：xcode-select --install 后重试。"
fi
ok "Xcode SDK：$(xcrun --show-sdk-path)"

# ── 3. Node / pnpm ─────────────────────────────────────────────────
command -v node >/dev/null 2>&1 || die "未找到 node。需要 Node.js ≥ 20（建议 20/22 LTS）。"
NODE_MAJOR="$(node -p "process.versions.node.split('.')[0]")"
[ "$NODE_MAJOR" -ge 20 ] || die "Node.js 版本过低（当前 $(node -v)），需要 ≥ 20。"
ok "Node：$(node -v)"

command -v pnpm >/dev/null 2>&1 \
  || die "未找到 pnpm。执行 npm install -g pnpm（或 corepack enable）后重试。"
PNPM_MAJOR="$(pnpm -v | cut -d. -f1)"
[ "$PNPM_MAJOR" -ge 9 ] || die "pnpm 版本过低（当前 $(pnpm -v)），需要 ≥ 9。"
ok "pnpm：$(pnpm -v)"

# ── 4. 前端依赖 ────────────────────────────────────────────────────
if [ ! -d "$ROOT/node_modules" ]; then
  info "安装前端依赖（pnpm install）…"
  pnpm install || die "pnpm install 失败：请检查网络 / registry 配置。"
else
  tip "node_modules 已存在，跳过安装（需要重装：rm -rf node_modules && pnpm install）"
fi

# ── 5. 图标资源检查（打包阶段必需）────────────────────────────────
ICONS="$ROOT/src-tauri/icons"
if [ ! -f "$ICONS/icon.icns" ] || [ ! -f "$ICONS/icon.ico" ]; then
  tip "src-tauri/icons/ 图标不完整（缺 icon.icns / icon.ico），打包可能失败。"
  tip "可先准备一张 ≥1024×1024 的 PNG，然后执行：pnpm tauri icon assets/app-icon.png"
fi

# ── 6. 构建 ────────────────────────────────────────────────────────
START_TS="$(date +%s)"
info "开始构建：pnpm tauri build $*"
if ! pnpm tauri build "$@"; then
  die "构建失败。排查建议：
  1) 完整报错见上方日志；
  2) 常见问题见 docs/构建与发布.md#常见构建问题；
  3) 先跑 pnpm tauri dev 确认前端与 Rust 都能启动；
  4) 打包阶段报错多与图标、签名、bundle targets 有关。"
fi
DURATION="$(( $(date +%s) - START_TS ))s"
ok "构建完成（耗时 ${DURATION}）"

# ── 7. 打印产物路径 ────────────────────────────────────────────────
BUNDLE="$ROOT/src-tauri/target/release/bundle"
FOUND=0

if [ -d "$BUNDLE/macos" ]; then
  for app in "$BUNDLE/macos"/*.app; do
    [ -e "$app" ] || continue
    echo "  .app → $app"
    FOUND=1
  done
fi
if [ -d "$BUNDLE/dmg" ]; then
  for dmg in "$BUNDLE/dmg"/*.dmg; do
    [ -e "$dmg" ] || continue
    echo "  dmg  → $dmg（$(du -h "$dmg" | cut -f1)）"
    FOUND=1
  done
fi

[ "$FOUND" -eq 1 ] || die "构建结束但未找到产物（检查 $BUNDLE 目录；无签名配置时也应产出 dmg/app）。"

echo
ok "产物位于：$BUNDLE"
echo "  ${C_DIM}未签名 App 首次打开：右键 →「打开」，或 xattr -cr \"/Applications/互联.app\"${C_RESET}"
echo "  ${C_DIM}安装体验：打开 dmg，把「互联」拖进「应用程序」${C_RESET}"
echo "  ${C_DIM}真实链路自检（需已配置账号）："/Applications/互联.app/Contents/MacOS/互联" --selftest${C_RESET}"
