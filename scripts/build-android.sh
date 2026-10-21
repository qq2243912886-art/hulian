#!/usr/bin/env bash
# 互联 · Android 一键构建（debug 签名 APK）
# 用法：
#   bash scripts/build-android.sh             # 构建 debug APK
#   bash scripts/build-android.sh --release   # 构建 release APK（需先配置 release 签名）
# 前置（脚本会逐项检查并给出安装命令）：
#   JDK 17、Android SDK（platforms;android-35 / build-tools;35.0.0 / platform-tools）、
#   NDK 27.0.12077973、Rust stable + Android targets、Node ≥ 20 + pnpm。
# 产物：
#   src-tauri/gen/android/app/build/outputs/apk/**/**.apk
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# 目标版本（与 .github/workflows/build.yml、docs/构建与发布.md 保持一致）
NDK_VERSION="27.0.12077973"
ANDROID_API="35"
BUILD_TOOLS_VERSION="35.0.0"

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
case "$(uname -s)" in
  Darwin|Linux) ;;
  *) die "Android 构建请在 macOS / Linux（或 CI ubuntu-latest）上执行。" ;;
esac
info "目标平台：$(uname -s) $(uname -m)"

# ── 1. Rust（优先使用项目内 .toolchain）───────────────────────────
if ! command -v cargo >/dev/null 2>&1; then
  if [ -f "$ROOT/scripts/env.sh" ]; then
    # shellcheck disable=SC1091
    . "$ROOT/scripts/env.sh"
    info "已加载项目内 Rust 工具链（scripts/env.sh）"
  fi
fi
command -v cargo >/dev/null 2>&1 \
  || die "未找到 cargo。请安装 rustup（https://rustup.rs/），或先执行 source scripts/env.sh。"
command -v rustc >/dev/null 2>&1 || die "未找到 rustc（Rust 安装不完整，重装 rustup 后再试）。"
ok "Rust：$(rustc -V)"

# 安装 Android 需要的 4 个 Rust 目标（rustup 场景）
if command -v rustup >/dev/null 2>&1; then
  RUST_TARGETS="aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android"
  INSTALLED_TARGETS="$(rustup target list --installed 2>/dev/null || true)"
  for t in $RUST_TARGETS; do
    if ! printf '%s\n' "$INSTALLED_TARGETS" | grep -qx "$t"; then
      info "安装 Rust 目标：$t"
      rustup target add "$t" || die "安装 Rust 目标 $t 失败（检查网络 / rustup 配置）。"
    fi
  done
  ok "Rust Android 目标已就绪"
else
  tip "未找到 rustup，跳过 Rust Android 目标检查；若构建报 missing target，请安装 rustup 后重试。"
fi

# ── 2. Node / pnpm ─────────────────────────────────────────────────
command -v node >/dev/null 2>&1 || die "未找到 node。需要 Node.js ≥ 20（建议 20/22 LTS）。"
NODE_MAJOR="$(node -p "process.versions.node.split('.')[0]")"
[ "$NODE_MAJOR" -ge 20 ] || die "Node.js 版本过低（当前 $(node -v)），需要 ≥ 20。"
ok "Node：$(node -v)"

command -v pnpm >/dev/null 2>&1 \
  || die "未找到 pnpm。执行 npm install -g pnpm（或 corepack enable）后重试。"
ok "pnpm：$(pnpm -v)"

# ── 3. JDK 17 ──────────────────────────────────────────────────────
command -v java >/dev/null 2>&1 \
  || die "未找到 java。请安装 JDK 17（推荐 Temurin 17）并设置 JAVA_HOME。"
JAVA_RAW="$(java -version 2>&1 | awk -F'"' '/version/ {print $2; exit}')"
[ -n "$JAVA_RAW" ] || die "无法解析 java 版本（java -version 输出异常）。"
JAVA_MAJOR="$(printf '%s' "$JAVA_RAW" | cut -d. -f1)"
if [ "$JAVA_MAJOR" = "1" ]; then
  JAVA_MAJOR="$(printf '%s' "$JAVA_RAW" | cut -d. -f2)"
fi
[ "$JAVA_MAJOR" -eq 17 ] || die "需要 JDK 17（当前 $(java -version 2>&1 | head -n 1)）。
macOS 示例：
  export JAVA_HOME=\"\$(/usr/libexec/java_home -v 17)\"
  export PATH=\"\$JAVA_HOME/bin:\$PATH\""
ok "JDK：$(java -version 2>&1 | head -n 1)"
if [ -z "${JAVA_HOME:-}" ]; then
  tip "未设置 JAVA_HOME（Gradle 可能仍能通过 PATH 找到 java，但建议设置：export JAVA_HOME=<JDK17 路径>）"
fi

# ── 4. Android SDK / NDK ───────────────────────────────────────────
SDK_ROOT="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
if [ -z "$SDK_ROOT" ]; then
  for candidate in "$HOME/Library/Android/sdk" "$HOME/Android/Sdk"; do
    if [ -d "$candidate" ]; then
      SDK_ROOT="$candidate"
      break
    fi
  done
fi
[ -n "$SDK_ROOT" ] || die "未找到 Android SDK。请安装 Android SDK 并设置：
  export ANDROID_HOME=\"\$HOME/Library/Android/sdk\"   # 按实际路径修改
  export ANDROID_SDK_ROOT=\"\$ANDROID_HOME\""
[ -d "$SDK_ROOT" ] || die "ANDROID_HOME 指向的目录不存在：$SDK_ROOT"
export ANDROID_HOME="$SDK_ROOT"
export ANDROID_SDK_ROOT="$SDK_ROOT"

MISSING=""
[ -d "$SDK_ROOT/platforms/android-$ANDROID_API" ] || MISSING="$MISSING platforms;android-$ANDROID_API"
[ -d "$SDK_ROOT/build-tools/$BUILD_TOOLS_VERSION" ] || MISSING="$MISSING build-tools;$BUILD_TOOLS_VERSION"
if [ -n "$MISSING" ]; then
  die "Android SDK 缺少组件：$MISSING
请先安装（sdkmanager 在 \$ANDROID_HOME/cmdline-tools/latest/bin/）：
  export PATH=\"\$ANDROID_HOME/cmdline-tools/latest/bin:\$PATH\"
  sdkmanager --licenses
  sdkmanager \"platform-tools\" \"platforms;android-$ANDROID_API\" \"build-tools;$BUILD_TOOLS_VERSION\" \"ndk;$NDK_VERSION\""
fi

NDK_ROOT="$SDK_ROOT/ndk/$NDK_VERSION"
[ -d "$NDK_ROOT" ] || die "未找到 NDK $NDK_VERSION（预期路径：$NDK_ROOT）。安装：
  sdkmanager \"ndk;$NDK_VERSION\""
# Tauri / Gradle 使用 NDK_HOME 与 ANDROID_NDK_HOME 定位 NDK，两个都导出最稳
export NDK_HOME="$NDK_ROOT"
export ANDROID_NDK_HOME="$NDK_ROOT"
export PATH="$SDK_ROOT/platform-tools:$SDK_ROOT/cmdline-tools/latest/bin:$PATH"
ok "Android SDK：$SDK_ROOT"
ok "Android NDK：$NDK_ROOT"

# ── 5. 前端依赖 ────────────────────────────────────────────────────
if [ ! -d "$ROOT/node_modules" ]; then
  info "安装前端依赖（pnpm install）…"
  pnpm install || die "pnpm install 失败：请检查网络 / registry 配置。"
else
  tip "node_modules 已存在，跳过安装（需要重装：rm -rf node_modules && pnpm install）"
fi

# ── 6. 图标提示（Android launcher 图标来自 tauri.conf 配置）────────
if [ ! -f "$ROOT/src-tauri/icons/icon.ico" ] && [ ! -f "$ROOT/src-tauri/icons/icon.png" ]; then
  tip "src-tauri/icons/ 图标不完整，Android 构建可能失败：pnpm tauri icon assets/app-icon.png"
fi

# ── 7. Android 工程初始化（幂等：已存在则跳过）─────────────────────
GEN_ANDROID="$ROOT/src-tauri/gen/android"
if [ -d "$GEN_ANDROID" ]; then
  info "已存在 Android 工程 src-tauri/gen/android，跳过 init（需要重建：rm -rf src-tauri/gen/android 后重跑）"
else
  info "初始化 Android 工程：NONINTERACTIVE=1 pnpm tauri android init --ci"
  # --ci：跳过交互提问（tauri android init --help 可见该参数；CI 环境变量 CI=1 等价）
  NONINTERACTIVE=1 pnpm tauri android init --ci \
    || die "android init 失败。常见原因：JDK/SDK/NDK 环境变量未生效、图标缺失、Rust 目标缺失。"
fi

# ── 8. debug keystore（debug 包签名；Gradle 默认也会自动生成）──────
KEYSTORE="$HOME/.android/debug.keystore"
if [ -f "$KEYSTORE" ]; then
  ok "debug keystore：$KEYSTORE"
else
  command -v keytool >/dev/null 2>&1 || die "未找到 keytool（随 JDK 提供）。请确认安装的是完整 JDK 17。"
  mkdir -p "$HOME/.android"
  info "生成 debug keystore：$KEYSTORE"
  keytool -genkeypair -v \
    -keystore "$KEYSTORE" -storepass android -alias androiddebugkey -keypass android \
    -keyalg RSA -keysize 2048 -validity 10000 \
    -dname "CN=Android Debug,O=Android,C=US" -noprompt >/dev/null 2>&1 \
    || die "生成 debug keystore 失败（检查 ~/.android 写权限）。"
  ok "debug keystore 已生成"
fi

# ── 9. 构建 ────────────────────────────────────────────────────────
BUILD_MODE="debug"
BUILD_ARGS=(--debug --apk)
case "${1:-}" in
  "")
    ;;
  --release)
    BUILD_MODE="release"
    BUILD_ARGS=(--apk)
    tip "release 构建需要已配置 release 签名（keystore 不要提交到仓库）。"
    ;;
  *)
    die "未知参数：$1。可用：--release（不带参数 = debug）。"
    ;;
esac

START_TS="$(date +%s)"
info "开始构建 Android ${BUILD_MODE} APK：pnpm tauri android build ${BUILD_ARGS[*]}"
if ! pnpm tauri android build "${BUILD_ARGS[@]}"; then
  die "Android 构建失败。排查建议：
  1) 完整报错见上方日志（Gradle / Rust / NDK）；
  2) 常见问题见 docs/构建与发布.md#常见构建问题；
  3) 确认 JAVA_HOME=JDK17、ANDROID_HOME、NDK_HOME 已导出；
  4) 首次构建需下载 Gradle 与依赖，网络差时容易失败，可重试。"
fi
DURATION="$(( $(date +%s) - START_TS ))s"
ok "构建完成（耗时 ${DURATION}）"

# ── 10. 打印产物路径 ───────────────────────────────────────────────
APK_ROOT="$GEN_ANDROID/app/build/outputs/apk"
[ -d "$APK_ROOT" ] || die "未找到 APK 输出目录：$APK_ROOT（构建日志是否显示成功？）"

FOUND=0
while IFS= read -r apk; do
  [ -n "$apk" ] || continue
  echo "  APK → $apk（$(du -h "$apk" | cut -f1)）"
  FOUND=1
done < <(find "$APK_ROOT" -type f -name '*.apk' | sort)

[ "$FOUND" -eq 1 ] || die "未找到 APK 文件：$APK_ROOT"

echo
ok "产物位于：$APK_ROOT"
echo "  ${C_DIM}安装到已连接设备：adb install -r <上面的 APK 路径>${C_RESET}"
echo "  ${C_DIM}debug 签名仅用于测试，不适合分发；正式发布需配置 release keystore 并构建 AAB/APK${C_RESET}"
echo "  ${C_DIM}首次安装后请真机验证：上传（系统选择器会先复制到 App 缓存）、下载、后台存活${C_RESET}"
