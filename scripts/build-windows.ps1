#Requires -Version 5.1
<#
.SYNOPSIS
    互联 · Windows 一键构建（NSIS 安装包）

.DESCRIPTION
    前置：Node.js >= 20、pnpm、Rust MSVC 工具链、Visual Studio Build Tools（C++ 桌面开发）。
    构建命令：pnpm tauri build --bundles nsis
    产物：src-tauri\target\release\bundle\nsis\互联_<版本>_x64-setup.exe

.PARAMETER SkipInstall
    跳过 pnpm install（node_modules 已存在时）。

.PARAMETER DebugBuild
    构建 debug 版本（调试用，不做正式分发）。

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1 -SkipInstall
#>
[CmdletBinding()]
param(
    [switch]$SkipInstall,
    [switch]$DebugBuild
)

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
Push-Location $Root

function Write-Step([string]$Message) {
    Write-Host "==> $Message" -ForegroundColor Green
}

function Write-Tip([string]$Message) {
    Write-Host "[!] $Message" -ForegroundColor Yellow
}

function Stop-Build([string]$Message) {
    Write-Host "[x] $Message" -ForegroundColor Red
    exit 1
}

try {
    # ── 0. 平台检查 ──────────────────────────────────────────────
    if ($env:OS -ne 'Windows_NT') {
        Stop-Build "本脚本仅用于 Windows。macOS 请用 scripts/build-macos.sh；Android 请用 scripts/build-android.sh。"
    }
    Write-Step "目标平台：Windows $([System.Environment]::OSVersion.Version) ($env:PROCESSOR_ARCHITECTURE)"

    # ── 1. Rust（优先使用项目内 .toolchain）─────────────────────
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        $cargoBin = Join-Path $Root '.toolchain\cargo\bin'
        if (Test-Path $cargoBin) {
            $env:RUSTUP_HOME = Join-Path $Root '.toolchain\rustup'
            $env:CARGO_HOME = Join-Path $Root '.toolchain\cargo'
            $env:PATH = "$cargoBin;$env:PATH"
            Write-Step "已加载项目内 Rust 工具链（.toolchain）"
        }
    }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Stop-Build "未找到 cargo。请安装 rustup（https://rustup.rs/），并选择 MSVC 工具链：rustup default stable-x86_64-pc-windows-msvc"
    }
    $rustcVersion = ((& rustc -V) -join ' ').Trim()
    if ($rustcVersion -notmatch 'msvc') {
        Write-Tip "当前 Rust 可能不是 MSVC 工具链（$rustcVersion）。Windows 构建需要 *-pc-windows-msvc：rustup default stable-x86_64-pc-windows-msvc"
    }
    Write-Host "    Rust: $rustcVersion"

    # ── 2. Node / pnpm ───────────────────────────────────────────
    if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
        Stop-Build "未找到 node。需要 Node.js >= 20（建议 20/22 LTS）。"
    }
    $nodeVersion = ((& node -p "process.versions.node") -join '').Trim()
    $nodeMajor = [int]($nodeVersion.Split('.')[0])
    if ($nodeMajor -lt 20) {
        Stop-Build "Node.js 版本过低（当前 $nodeVersion），需要 >= 20。"
    }
    Write-Host "    Node: v$nodeVersion"

    if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) {
        Stop-Build "未找到 pnpm。执行：npm install -g pnpm（或 corepack enable）后重试。"
    }
    $pnpmVersion = ((& pnpm -v) -join '').Trim()
    Write-Host "    pnpm: $pnpmVersion"

    # ── 3. MSVC 工具链提示（真正链接由 cargo 调用 link.exe）──────
    $vsWhereRoot = ${env:ProgramFiles(x86)}
    if ($vsWhereRoot) {
        $vswhere = Join-Path $vsWhereRoot 'Microsoft Visual Studio\Installer\vswhere.exe'
        if (Test-Path $vswhere) {
            $msvcPath = ((& $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath) -join '').Trim()
            if ([string]::IsNullOrWhiteSpace($msvcPath)) {
                Write-Tip "未检测到 MSVC C++ 工具链。请安装 Visual Studio Build Tools，并勾选「使用 C++ 的桌面开发」。"
            }
            else {
                Write-Host "    MSVC: $msvcPath"
            }
        }
        else {
            Write-Tip "未找到 vswhere（可能未安装 Visual Studio Build Tools）。若链接阶段报 link.exe 错误，请安装 VS Build Tools 的 C++ 桌面开发组件。"
        }
    }

    # ── 4. WebView2 运行时提示 ───────────────────────────────────
    $webviewCandidates = @()
    if (${env:ProgramFiles(x86)}) { $webviewCandidates += (Join-Path ${env:ProgramFiles(x86)} 'Microsoft\EdgeWebView\Application') }
    if ($env:ProgramFiles) { $webviewCandidates += (Join-Path $env:ProgramFiles 'Microsoft\EdgeWebView\Application') }
    $hasWebView2 = $false
    foreach ($candidate in $webviewCandidates) {
        if (Test-Path $candidate) { $hasWebView2 = $true }
    }
    if ($hasWebView2) {
        Write-Host "    WebView2: 已安装"
    }
    else {
        Write-Tip "未检测到 WebView2 运行时（Win10 1803+ 通常自带；缺失时 App 会白屏，请安装 Evergreen Runtime）。"
    }

    # ── 5. 前端依赖 ──────────────────────────────────────────────
    $nodeModules = Join-Path $Root 'node_modules'
    if ($SkipInstall) {
        Write-Tip "已跳过 pnpm install（-SkipInstall）"
    }
    elseif (-not (Test-Path $nodeModules)) {
        Write-Step "安装前端依赖（pnpm install）..."
        & pnpm install
        if ($LASTEXITCODE -ne 0) {
            Stop-Build "pnpm install 失败：请检查网络 / registry 配置。"
        }
    }
    else {
        Write-Tip "node_modules 已存在，跳过安装（需要重装：Remove-Item -Recurse -Force node_modules 后重跑）。"
    }

    # ── 6. 图标资源检查（打包阶段必需）───────────────────────────
    $icoPath = Join-Path $Root 'src-tauri\icons\icon.ico'
    if (-not (Test-Path $icoPath)) {
        Write-Tip "src-tauri\icons\icon.ico 不存在，打包可能失败。先准备 >=1024x1024 的 PNG，再执行：pnpm tauri icon assets\icon.png"
    }

    # ── 7. 构建 ──────────────────────────────────────────────────
    # 说明：tauri.conf.json 的 bundle.targets 面向 macOS（app/dmg），
    # Windows 必须显式指定 nsis，否则可能不产出安装包。
    $pnpmArgs = @('tauri', 'build', '--bundles', 'nsis')
    if ($DebugBuild) { $pnpmArgs += '--debug' }
    Write-Step ("开始构建：pnpm " + ($pnpmArgs -join ' '))
    & pnpm @pnpmArgs
    if ($LASTEXITCODE -ne 0) {
        Stop-Build "构建失败。排查建议：1) 完整报错见上方日志；2) 常见问题见 docs/构建与发布.md 的「常见构建问题」；3) 建议在「x64 Native Tools Command Prompt for VS」中运行；4) 打包阶段报错多与图标、MSVC、bundle targets 有关。"
    }
    Write-Step "构建完成"

    # ── 8. 打印产物路径 ──────────────────────────────────────────
    if ($DebugBuild) {
        $bundleRoot = Join-Path $Root 'src-tauri\target\debug\bundle'
    }
    else {
        $bundleRoot = Join-Path $Root 'src-tauri\target\release\bundle'
    }

    $found = @()
    foreach ($sub in @('nsis', 'msi')) {
        $dir = Join-Path $bundleRoot $sub
        if (Test-Path $dir) {
            $items = Get-ChildItem -Path $dir -File -ErrorAction SilentlyContinue |
                Where-Object { $_.Extension -eq '.exe' -or $_.Extension -eq '.msi' }
            if ($items) { $found += $items }
        }
    }

    if ($found.Count -eq 0) {
        Stop-Build "未找到安装包产物（$bundleRoot\nsis 或 msi）。请检查构建日志；若使用 -DebugBuild，安装包可能不生成，请改用默认 release 构建。"
    }

    Write-Host ""
    Write-Step "产物："
    foreach ($file in $found) {
        Write-Host ("  {0}  ({1:N1} MB)" -f $file.FullName, ($file.Length / 1MB))
    }

    Write-Host ""
    Write-Host "提示：" -ForegroundColor Cyan
    Write-Host "  - 未签名安装包会触发 SmartScreen：「更多信息」→「仍要运行」；正式分发需代码签名证书。"
    Write-Host "  - 运行依赖 WebView2：Win10 1803+ / Win11 自带；缺失时安装 Evergreen Runtime。"
    Write-Host "  - 真实链路自检（需已配置账号）：'互联.exe --selftest'。"
    Write-Host "  - 改版本号需同步：package.json、src-tauri/tauri.conf.json、src-tauri/Cargo.toml。"
}
finally {
    Pop-Location
}
