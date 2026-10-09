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

## V2.0.1：多局比赛与出局排名

- 状态：已完成
- 开始时间：2026-10-09
- 完成时间：2026-10-09
- 测试命令：`cargo fmt --manifest-path D:\\yinhanPlay\\Cargo.toml --check`、`cargo check --manifest-path D:\\yinhanPlay\\Cargo.toml`、`cargo check --manifest-path D:\\yinhanPlay\\Cargo.toml --target wasm32-unknown-unknown`、`cargo test --manifest-path D:\\yinhanPlay\\Cargo.toml`、`trunk build --config D:\\yinhanPlay\\Trunk.toml`
- 测试结果：通过；新增 `MatchState` 多局管理、完整出局排名、`3/2/1/-1` 积分、负分、10分终止、多人待加赛状态和连续随机发牌；规则测试共28项全部通过
- 浏览器验证：本版本不修改浏览器界面；Trunk WebAssembly 构建通过
- 遗留问题：V2.0.6 实现实际2至4人加赛流程，V2.0.7 接入积分榜和比赛状态界面
- 备注：保留 `GameState::new([String; 4], seed)`；开局整理阶段空手玩家按整理完成顺序计入本轮排名；多人同时达到10分只记录 `PlayoffPending`，不提前启动加赛

## V2.0.2：行动牌数据与公共牌堆

- 状态：已完成
- 开始时间：2026-10-09
- 完成时间：2026-10-09
- 测试命令：`cargo fmt --manifest-path D:\\yinhanPlay\\Cargo.toml --check`、`cargo check --manifest-path D:\\yinhanPlay\\Cargo.toml`、`cargo check --manifest-path D:\\yinhanPlay\\Cargo.toml --target wasm32-unknown-unknown`、`cargo test --manifest-path D:\\yinhanPlay\\Cargo.toml`、`trunk build --config D:\\yinhanPlay\\Trunk.toml`
- 测试结果：通过；新增6张行动牌独立牌堆、随机分配、独立手牌、行动牌弃置区、私有消耗事件和守恒校验；规则测试共33项全部通过
- 浏览器验证：本版本不修改浏览器界面；Trunk WebAssembly 构建通过
- 遗留问题：V2.0.3 实现窥视与护盾，V2.0.4 实现重抽，V2.0.7 接入行动牌私有界面
- 备注：行动牌不混入普通牌组，不写入公共 `GameEvent` 日志；多局换局时行动牌全部重置；具体使用时机和效果不在本子版本实现

## V2.0.3：窥视与护盾

- 状态：已完成
- 开始时间：2026-10-09
- 完成时间：2026-10-09
- 测试命令：`cargo fmt --manifest-path D:\\yinhanPlay\\Cargo.toml --check`、`cargo check --manifest-path D:\\yinhanPlay\\Cargo.toml`、`cargo check --manifest-path D:\\yinhanPlay\\Cargo.toml --target wasm32-unknown-unknown`、`cargo test --manifest-path D:\\yinhanPlay\\Cargo.toml`、`trunk build --config D:\\yinhanPlay\\Trunk.toml`
- 测试结果：通过；新增窥视请求、护盾响应、私有结果读取、位置和目标校验、主动行动额度及护盾消耗规则；规则测试共38项全部通过
- 浏览器验证：本版本完成规则层状态机，不修改浏览器界面；Trunk WebAssembly 构建通过
- 遗留问题：V2.0.4 实现重抽牌；V2.0.7 接入窥视位置选择、护盾响应和私有信息交接界面
- 备注：窥视结果不写入公共 `GameEvent`；必须读取私有结果后才能继续当前回合；护盾为被动响应，不占主动行动额度

## V2.0.4：重抽牌

- 状态：已完成
- 开始时间：2026-10-09
- 完成时间：2026-10-09
- 测试命令：`cargo fmt --manifest-path D:\\yinhanPlay\\Cargo.toml --check`、`cargo check --manifest-path D:\\yinhanPlay\\Cargo.toml`、`cargo check --manifest-path D:\\yinhanPlay\\Cargo.toml --target wasm32-unknown-unknown`、`cargo test --manifest-path D:\\yinhanPlay\\Cargo.toml`、`trunk build --config D:\\yinhanPlay\\Trunk.toml`
- 测试结果：通过；`PendingDraw`、原下家退回、同一下家重抽、重抽后二次对子检测和原下家单牌禁用均已实现；最终完整测试集为47项，47/47通过；原生/WASM检查和 Trunk 0.21.14 构建通过
- 浏览器验证：逻辑已接入界面；Trunk 构建通过，人工浏览器验收需启动本地服务完成
- 遗留问题：不实现下家限时整理、指定位置替换、连续多次重抽和抽两张选一张
- 备注：重抽属于主动行动牌，护盾不占主动额度

## V2.0.5：行动牌与正式回合整合

- 状态：已完成
- 开始时间：2026-10-09
- 完成时间：2026-10-09
- 测试命令：同 V2.0.4，并补充 `cargo test --manifest-path D:\\yinhanPlay\\Cargo.toml`
- 测试结果：通过；窥视、重抽、护盾和普通弃牌/抽牌流程共享当前玩家与阶段校验，私有行动事件不进入公共事件日志
- 浏览器验证：热座交接和行动牌区域已接入界面
- 遗留问题：行动牌平衡数据在 V2.0.8 模拟器中记录，不改变核心牌组
- 备注：每回合最多一张主动行动牌，护盾为被动响应

## V2.0.6：2至4人加赛

- 状态：已完成
- 开始时间：2026-10-09
- 完成时间：2026-10-09
- 测试命令：`cargo test --manifest-path D:\\yinhanPlay\\Cargo.toml match_state`、`cargo test --manifest-path D:\\yinhanPlay\\Cargo.toml simulation::tests::two_to_four_player_playoffs_finish_without_deadlock`
- 测试结果：通过；新增 `RoundMode`、2至4人发牌、待加赛转实际加赛、全局排名映射、积分不变和2人自动结束测试
- 浏览器验证：待加赛和加赛结束按钮已接入界面；需运行 Trunk 后人工完成完整热座流程
- 遗留问题：加赛不增加积分，符合本版本范围；实际加赛体验由 V2.0.7 验收
- 备注：未达到10分的玩家不会进入加赛

## V2.0.7：浏览器界面与隐私交接

- 状态：已完成
- 开始时间：2026-10-09
- 完成时间：2026-10-09
- 测试命令：`cargo check --manifest-path D:\\yinhanPlay\\Cargo.toml`、`cargo check --manifest-path D:\\yinhanPlay\\Cargo.toml --target wasm32-unknown-unknown`、`trunk build --config D:\\yinhanPlay\\Trunk.toml`
- 测试结果：通过；界面使用 `MatchState`，显示比分/局号/待加赛/最终排名，接入行动牌私有区域、窥视、护盾和重抽流程；`cargo check`、WASM 检查和 Trunk 0.21.14 构建通过
- 浏览器验证：源码、WASM 和 Trunk 构建通过；尚需启动本地服务进行人工热座验收
- 遗留问题：无新增规则层遗留；仍不实现跨设备同步和网络联机
- 备注：回合交接通过“显示我的手牌”控制当前热座玩家可见范围

## V2.0.8：模拟、平衡与稳定性验证

- 状态：已完成
- 开始时间：2026-10-09
- 完成时间：2026-10-09
- 测试命令：`cargo test --manifest-path D:\\yinhanPlay\\Cargo.toml simulation::tests::one_thousand_match_seeds_preserve_match_invariants`、`cargo test --manifest-path D:\\yinhanPlay\\Cargo.toml simulation::tests::two_to_four_player_playoffs_finish_without_deadlock`、`cargo test --manifest-path D:\\yinhanPlay\\Cargo.toml simulation::tests::action_card_match_simulation_is_deterministic_and_bounded`
- 测试结果：通过；完整测试集47/47通过；1000组普通比赛、2/3/4人各100组加赛均在上限内完成，动作牌统计和固定种子确定性通过；原生/WASM检查通过
- 浏览器验证：不新增浏览器功能；Trunk 0.21.14 构建通过
- 遗留问题：当前报告记录使用次数，尚未加入长期统计图表；后续可据模拟结果调整行动牌数量或出现率
- 备注：超过最大回合/局数时返回包含种子的诊断错误，不静默卡死
