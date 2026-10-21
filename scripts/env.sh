#!/bin/sh
# 统一构建环境：Rust / npm 缓存全部约束在项目目录内。
# 用法：source scripts/env.sh （在项目根目录或任意位置均可）
_hulian_src="${BASH_SOURCE[0]:-$0}"
if [ -f "$_hulian_src" ]; then
  _hulian_root="$(cd "$(dirname "$_hulian_src")/.." && pwd)"
else
  _hulian_root="$(pwd)"
fi
ROOT="$_hulian_root"
export ROOT
export RUSTUP_HOME="$ROOT/.toolchain/rustup"
export CARGO_HOME="$ROOT/.toolchain/cargo"
export PATH="$CARGO_HOME/bin:$PATH"
export XDG_CACHE_HOME="$ROOT/.cache"
export npm_config_cache="$ROOT/.cache/npm"
unset _hulian_src _hulian_root
