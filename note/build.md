# 构建环境

## 前端与桌面工具链

- Node.js、npm、pnpm 使用系统已有安装。
- Tauri CLI 使用项目内 `@tauri-apps/cli`，不需要全局 `cargo-tauri`。
- MSVC Rust 位于 `F:\DevCache\toolchains\1.95-x86_64-pc-windows-msvc\bin`。
- Cargo/Rustup 缓存位于 `F:\DevCache\cargo` 与 `F:\DevCache\rustup`。
- MSVC 初始化脚本位于 `F:\soft\BuildTools\devcmd.ps1`。

常用检查：

```bash
pnpm install
pnpm build
```

Rust 检查需要在 PowerShell 中先执行 `F:\soft\BuildTools\devcmd.ps1`，再把 MSVC Rust 的 `bin` 放到 PATH 前部，然后在 `src-tauri` 下执行 `cargo check`、`cargo fmt --check` 或 `cargo clippy -- -D warnings`。

## NSIS 离线缓存

Tauri 默认使用用户缓存中的 `tauri/NSIS`。本项目在 `tauri.conf.json` 设置了 `bundle.useLocalToolsDir=true`，因此实际路径是 Cargo `target_directory/.tauri/NSIS`，即 `src-tauri/target/.tauri/NSIS`。官方源码：

- https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-bundler/src/bundle/windows/nsis/mod.rs
- https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-bundler/src/bundle/windows/util.rs
- https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/src/interface/mod.rs

首次构建使用 GitHub CLI 获取官方文件：

```bash
gh release download nsis-3.13 --repo tauri-apps/binary-releases --pattern nsis-3.13.zip --dir TEMP/nsis --clobber
gh release download nsis_tauri_utils-v0.5.3 --repo tauri-apps/nsis-tauri-utils --pattern nsis_tauri_utils.dll --dir TEMP/nsis --clobber
```

可先将 `nsis-3.13` 解压到 `TEMP/build-cache/tauri/NSIS`，并将 DLL 放到 `TEMP/build-cache/tauri/NSIS/Plugins/x86-unicode/additional/nsis_tauri_utils.dll`，再完整复制到 `src-tauri/target/.tauri/NSIS`。

已核对的官方 SHA-1：

- `nsis-3.13.zip`: `DB14F8AF2EA346B786C6AE19343F61AAF5E09B8E`
- `NSIS.exe`: `6D45D4E8CCF18FE1FDAC9164058F356102A2ECF2`
- `nsis_tauri_utils.dll`: `75197FEE3C6A814FE035788D1C34EAD39349B860`

缓存就绪后直接构建，不需要设置 `NSIS_PATH` 或重定向 `LOCALAPPDATA`：

```powershell
pnpm tauri build
```

## 便携版

Release 构建成功后执行：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/package-portable.ps1 -Version 0.1.0
```

脚本将安装包与 `release/SoMarkClient-CE-0.1.0-portable.zip` 收集到 `release/`。便携版通过 EXE 同目录的 `portable.flag` 识别，数据写入便携目录，API Key 仅保留在内存中。
