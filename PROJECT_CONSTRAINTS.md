# 项目开发约束

## 文件修改范围

- 代码、配置、文档、测试、资源和构建产物只能位于 `D:\yinhanPlay` 内。
- 修改项目外的任何文件、目录或配置前，必须先向用户说明目标、原因和具体路径，并获得明确同意。
- 默认禁止修改 C 盘内容，尤其是 `C:\Users\yinhan\.rustup`、`C:\Users\yinhan\.cargo`、系统环境变量和全局工具配置。
- 不执行会自动写入项目外目录的安装、升级、清理或配置命令。
- 不使用全局 Cargo/Rust 工具安装作为项目实施步骤；如工具链操作必然写入项目外目录，必须先暂停并询问用户。

## 依赖与网络

- 优先使用 `D:\yinhanPlay\.cargo-home`、`D:\yinhanPlay\tools` 和 `D:\yinhanPlay\target`。
- 任何下载操作必须先确认下载目标和缓存位置；不能因为设置 `CARGO_HOME` 就默认认为 Rustup 工具链也位于项目目录。
- `wasm32-unknown-unknown` 的 Rust 标准库由 Rustup 管理，当前系统 Rustup 位于 C 盘；安装该目标前必须获得用户明确授权。

## Rust 构建工具链

- Rust项目默认使用 MSVC 工具链：`stable-x86_64-pc-windows-msvc`。
- Rust项目的检查、测试、原生构建和 WebAssembly 构建均优先通过 MSVC Rust工具链执行。
- 不切换到 GNU/MinGW Rust目标，不使用 `x86_64-pc-windows-gnu` 作为项目构建目标。
- WABT 等独立 C/C++ 辅助工具可以使用 MinGW 构建，但不得改变 Rust项目的 MSVC 构建约束。

## 当前诊断结论

- 正确的浏览器编译目标是 `wasm32-unknown-unknown`。
- 当前系统工具链为 `stable-x86_64-pc-windows-msvc`，Rust/Cargo 本身可运行。
- `wasm32-unknown-unknown` 已由用户安装并通过 `rustc --target wasm32-unknown-unknown --version` 验证。
- `wasm32-wasi` 不是当前浏览器目标；现代 WASI 目标名称为 `wasm32-wasip1`，本项目暂不需要 WASI。
- 从本环境访问 `static.rust-lang.org:443` 和 `index.crates.io:443` 均失败，因此暂不执行依赖安装。
