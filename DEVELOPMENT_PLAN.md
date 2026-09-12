# 第一版本分阶段开发记录

## 开发规则

每个子版本必须依次完成实现、自动化测试、人工验收和状态记录，未完成当前子版本前不进入下一个子版本。

所有代码、配置、文档和项目依赖操作只能在 `D:\\yinhanPlay` 内进行。修改项目外文件、C盘内容、系统环境变量或全局 Rust/Cargo 配置前，必须先获得用户明确同意。完整约束见 `PROJECT_CONSTRAINTS.md`。

Rust项目构建统一优先使用 `stable-x86_64-pc-windows-msvc`；MinGW仅用于独立的WABT辅助工具构建。

## V0.1：项目与构建环境

- 状态：已完成
- 开始时间：2026-09-10
- 完成时间：
- 测试命令：`cargo fmt --manifest-path D:\\yinhanPlay\\Cargo.toml`、`cargo check --manifest-path D:\\yinhanPlay\\Cargo.toml`、`cargo test --manifest-path D:\\yinhanPlay\\Cargo.toml`
- 测试结果：通过；Cargo项目、源码目录、系统Rust工具链配置和基础测试已创建
- 浏览器验证：等待 `wasm32-unknown-unknown` 和 Trunk
- 遗留问题：
- 备注：使用系统已有 Rust 工具链

## V0.2：扑克牌数据模型

- 状态：已完成
- 开始时间：
- 完成时间：
- 测试命令：`cargo test card --manifest-path D:\\yinhanPlay\\Cargo.toml`
- 测试结果：通过；四种花色、A到K、小王、唯一ID、显示文本和小王不可配对均已覆盖
- 浏览器验证：不适用
- 遗留问题：

## V0.3：洗牌与发牌

- 状态：已完成
- 开始时间：
- 完成时间：
- 测试命令：`cargo test deck --manifest-path D:\\yinhanPlay\\Cargo.toml`
- 测试结果：通过；53张牌唯一、13/13/13/14分配、固定随机种子可重复均已覆盖
- 浏览器验证：不适用
- 遗留问题：

## V0.4：对子识别与弃牌区

- 状态：已完成
- 开始时间：
- 完成时间：
- 测试命令：`cargo test game::tests::pair --manifest-path D:\\yinhanPlay\\Cargo.toml`
- 测试结果：通过；同点数配对、忽略花色、小王不参与配对和弃牌区逻辑已实现
- 浏览器验证：不适用
- 遗留问题：

## V0.5：开局整理与弃牌决策

- 状态：已完成
- 开始时间：
- 完成时间：
- 测试命令：`cargo test game::tests::setup --manifest-path D:\\yinhanPlay\\Cargo.toml`
- 测试结果：通过；玩家可以弃掉对子，也可以保留对子，四名玩家依次完成开局整理
- 浏览器验证：不适用
- 遗留问题：

## V0.6：基础回合系统

- 状态：已完成
- 开始时间：
- 完成时间：
- 测试命令：`cargo test game --manifest-path D:\\yinhanPlay\\Cargo.toml`
- 测试结果：通过；顺时针回合、当前玩家校验和空手玩家跳过逻辑已实现
- 浏览器验证：不适用
- 遗留问题：

## V0.7：随机抽牌规则

- 状态：已完成
- 开始时间：
- 完成时间：
- 测试命令：`cargo test game --manifest-path D:\\yinhanPlay\\Cargo.toml`
- 测试结果：通过；从下家随机抽牌、手牌转移、抽后对子处理状态已实现
- 浏览器验证：不适用
- 遗留问题：

## V0.8：终局判定

- 状态：已完成
- 开始时间：
- 完成时间：
- 测试命令：`cargo test game --manifest-path D:\\yinhanPlay\\Cargo.toml`
- 测试结果：通过；只剩小王时结束并识别输家，结束后禁止继续操作
- 浏览器验证：不适用
- 遗留问题：

## V0.9：规则模拟与稳定性验证

- 状态：已完成
- 开始时间：2026-09-10
- 完成时间：2026-09-10
- 测试命令：`cargo test --manifest-path D:\\yinhanPlay\\Cargo.toml`
- 测试结果：通过；18/18测试通过，64个固定种子完整对局均在10000回合内结束，牌数量、唯一性和小王不变量均通过
- 浏览器验证：不适用
- 遗留问题：无
- 备注：新增 `simulation` 模块和 `simulate_greedy`，支持最大回合保护

## V0.10：浏览器基础界面

- 状态：实现完成，待浏览器验收
- 开始时间：2026-09-10
- 完成时间：
- 测试命令：`cargo fmt --manifest-path D:\\yinhanPlay\\Cargo.toml --check`、`cargo check --target wasm32-unknown-unknown`、`cargo test`、`trunk build`
- 测试结果：界面源码、`index.html`、`Trunk.toml` 和 `NotoSansSC-Regular.otf` 中文字体已加入；格式检查、MSVC原生编译、WASM编译和18/18单元测试均通过；Trunk 0.21.14 构建成功并生成 `dist/index.html`、JavaScript 和 WASM 文件
- 浏览器验证：未完成
- 遗留问题：尚未完成浏览器人工验收；需要验证开始游戏、隐私交接、弃牌、抽牌和终局界面流程
- 阻塞原因：无；用户已在项目目录内提供并编译匹配的 `wasm-bindgen 0.2.128` CLI
- 诊断结论：正确目标为 `wasm32-unknown-unknown`；该目标已安装，系统工具链正常；WABT仅用于Wasm检查/转换，不能替代Rust标准库
- 环境约束：未修改 C 盘、Rustup、Cargo 全局配置或系统环境变量
- 下一步：执行 `trunk serve --config D:\\yinhanPlay\\Trunk.toml`，在浏览器完成一次人工热座试玩；通过后将状态改为“已完成”并记录浏览器结果

## V1.0：第一版可玩原型

- 状态：阻塞
- 开始时间：
- 完成时间：
- 测试命令：
- 测试结果：规则引擎已通过18/18测试；浏览器部分未完成
- 浏览器验证：未完成
- 遗留问题：依赖下载恢复后继续V0.10，再进行本地热座人工试玩
- 阻塞原因：依赖于V0.10浏览器构建环境
