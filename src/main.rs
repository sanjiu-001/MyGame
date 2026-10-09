use eframe::egui;
use yinhan_play::{
    GamePhase, MatchPhase, MatchState, PLAYER_COUNT, Pair, PeekResult, TurnPhase, XorShift64,
    find_pairs,
};

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::JsCast;

struct YinhanApp {
    names: [String; PLAYER_COUNT],
    seed_text: String,
    match_state: Option<MatchState>,
    random: XorShift64,
    hand_visible: bool,
    status: String,
}

impl YinhanApp {
    fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        Self::configure_chinese_font(&creation_context.egui_ctx);

        Self {
            names: ["玩家1", "玩家2", "玩家3", "玩家4"].map(str::to_owned),
            seed_text: "20260910".to_owned(),
            match_state: None,
            random: XorShift64::seeded(20260910),
            hand_visible: false,
            status: "请输入玩家名称后开始游戏".to_owned(),
        }
    }

    fn configure_chinese_font(context: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "noto_sans_sc".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/NotoSansSC-Regular.otf")).into(),
        );

        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, "noto_sans_sc".to_owned());
        }

        context.set_fonts(fonts);
    }

    fn start_game(&mut self) {
        let seed = self.seed_text.trim().parse::<u64>().unwrap_or(20260910);
        self.random = XorShift64::seeded(seed ^ 0xA5A5_5A5A);
        self.match_state = Some(MatchState::new(self.names.clone(), seed));
        self.hand_visible = false;
        self.status = "游戏已开始，请按顺序完成开局整理".to_owned();
    }

    fn restart(&mut self) {
        self.match_state = None;
        self.hand_visible = false;
        self.status = "请输入玩家名称后开始游戏".to_owned();
    }

    fn card_pair_buttons(ui: &mut egui::Ui, pairs: &[Pair]) -> Option<Pair> {
        let mut chosen = None;
        for pair in pairs {
            if ui
                .button(format!("弃掉 {} + {}", pair.first, pair.second))
                .clicked()
            {
                chosen = Some(*pair);
            }
        }
        chosen
    }

    fn show_setup(&mut self, ui: &mut egui::Ui) {
        let Some(match_state) = self.match_state.as_mut() else {
            return;
        };
        let game = match_state.game_mut();
        let player = game.setup_player;
        ui.heading(format!("开局整理：{}", game.players[player].name));
        ui.label(
            "当前玩家可以选择弃掉对子，也可以保留对子，然后点击完成整理。交接设备前请遮挡屏幕。",
        );
        ui.separator();
        ui.label("当前手牌：");
        ui.horizontal_wrapped(|ui| {
            for card in &game.players[player].hand {
                ui.monospace(card.to_string());
            }
        });
        ui.separator();

        let pairs = find_pairs(&game.players[player].hand);
        if pairs.is_empty() {
            ui.label("当前没有可弃置的对子。");
        } else if let Some(pair) = Self::card_pair_buttons(ui, &pairs) {
            match game.setup_discard(player, pair) {
                Ok(()) => self.status = format!("{} 已弃置一对牌", game.players[player].name),
                Err(error) => self.status = error.to_string(),
            }
        }

        if ui.button("完成整理并交给下一位玩家").clicked() {
            match game.finish_setup(player) {
                Ok(()) => {
                    self.hand_visible = false;
                    self.status = if game.phase == GamePhase::Playing {
                        "开局整理完成，游戏进入正式回合".to_owned()
                    } else {
                        "请交给下一位玩家".to_owned()
                    };
                }
                Err(error) => self.status = error.to_string(),
            }
        }
    }

    fn show_playing(&mut self, ui: &mut egui::Ui) {
        let Some(match_state) = self.match_state.as_mut() else {
            return;
        };
        let game = match_state.game_mut();
        if game.is_finished() {
            self.show_finished(ui);
            return;
        }
        let player = game.current_player;
        let player_name = game.players[player].name.clone();
        ui.heading(format!("{} 的回合", player_name));
        ui.label(format!("当前阶段：{:?}", game.turn_phase));

        if let Some(pending) = game.pending_peek {
            let target = pending.target;
            ui.label(format!(
                "请将设备交给 {}，选择是否使用护盾。",
                game.players[target].name
            ));
            let has_shield = game.players[target]
                .action_cards
                .iter()
                .any(|card| card.kind == yinhan_play::ActionCardKind::Shield);
            if has_shield && ui.button("使用护盾").clicked() {
                match game.respond_to_peek(target, true) {
                    Ok(()) => self.status = "护盾已生效，请交还设备".to_owned(),
                    Err(error) => self.status = error.to_string(),
                }
            }
            if ui.button("不使用护盾，允许查看").clicked() {
                match game.respond_to_peek(target, false) {
                    Ok(()) => self.status = "窥视已完成，请交还发起者".to_owned(),
                    Err(error) => self.status = error.to_string(),
                }
            }
            return;
        }

        if game.has_peek_result() {
            ui.label("窥视结果已准备好，请交还给发起窥视的玩家。");
            if ui.button("查看窥视结果").clicked() {
                match game.take_peek_result(player) {
                    Ok(PeekResult::Blocked { .. }) => {
                        self.status = "窥视被护盾阻挡".to_owned();
                        self.hand_visible = true;
                    }
                    Ok(PeekResult::Revealed { card, .. }) => {
                        self.status = format!("窥视到的牌是 {}", card);
                        self.hand_visible = true;
                    }
                    Err(error) => self.status = error.to_string(),
                }
            }
            return;
        }

        if !self.hand_visible {
            ui.label("请确认设备已交给当前玩家。");
            if ui.button("显示我的手牌").clicked() {
                self.hand_visible = true;
            }
            return;
        }

        ui.separator();
        ui.label("你的手牌：");
        ui.horizontal_wrapped(|ui| {
            for card in &game.players[player].hand {
                ui.monospace(card.to_string());
            }
        });
        let action_cards = game.players[player].action_cards.clone();
        ui.label("你的行动牌（仅当前热座玩家可见）：");
        ui.horizontal_wrapped(|ui| {
            for card in &action_cards {
                ui.monospace(card.to_string());
            }
        });
        ui.separator();

        match game.turn_phase {
            TurnPhase::AwaitingAction => {
                if !game.active_action_used
                    && action_cards
                        .iter()
                        .any(|card| card.kind == yinhan_play::ActionCardKind::Peek)
                {
                    ui.label("窥视牌：选择下家手牌中的位置");
                    if let Some(target) = game.peek_target(player) {
                        let hand_len = game.players[target].hand.len();
                        for position in 0..hand_len {
                            if ui.button(format!("窥视第 {} 张", position + 1)).clicked() {
                                match game.peek(player, position) {
                                    Ok(()) => {
                                        self.hand_visible = false;
                                        self.status = "已发起窥视，请交给下家回应".to_owned();
                                    }
                                    Err(error) => self.status = error.to_string(),
                                }
                            }
                        }
                    }
                }
                let pairs = find_pairs(&game.players[player].hand);
                if let Some(pair) = Self::card_pair_buttons(ui, &pairs) {
                    match game.discard_pair(player, pair) {
                        Ok(()) => {
                            self.hand_visible = false;
                            self.status = "已弃置对子，回合结束".to_owned();
                        }
                        Err(error) => self.status = error.to_string(),
                    }
                }
                if pairs.is_empty() {
                    ui.label("当前没有对子，必须抽取下家一张牌。");
                } else {
                    ui.label("你也可以保留对子，选择抽牌。");
                }
                if ui.button("从下家随机抽一张").clicked() {
                    match game.draw_from_next(player, &mut self.random) {
                        Ok(card) => {
                            if game.turn_phase == TurnPhase::AwaitingAction {
                                self.hand_visible = false;
                            }
                            self.status = format!("抽到了 {}", card);
                        }
                        Err(error) => self.status = error.to_string(),
                    }
                }
            }
            TurnPhase::ResolvingDraw => {
                ui.label("抽牌后可以弃掉对子或保留；若有重抽牌，也可以重抽一次。");
                if game
                    .pending_draw
                    .map(|draw| draw.can_redraw)
                    .unwrap_or(false)
                    && !game.active_action_used
                {
                    if ui.button("使用重抽牌").clicked() {
                        match game.redraw(player, &mut self.random) {
                            Ok(card) => {
                                self.status = format!("重抽得到 {}", card);
                                if game.turn_phase == TurnPhase::AwaitingAction
                                    || game.is_finished()
                                {
                                    self.hand_visible = false;
                                }
                            }
                            Err(error) => self.status = error.to_string(),
                        }
                    }
                }
                let pairs = find_pairs(&game.players[player].hand);
                if let Some(pair) = Self::card_pair_buttons(ui, &pairs) {
                    match game.discard_after_draw(player, pair) {
                        Ok(()) => {
                            self.hand_visible = false;
                            self.status = "已弃置抽到的对子，回合结束".to_owned();
                        }
                        Err(error) => self.status = error.to_string(),
                    }
                }
                if ui.button("保留对子并结束回合").clicked() {
                    match game.keep_drawn_pairs(player) {
                        Ok(()) => {
                            self.hand_visible = false;
                            self.status = "保留对子，回合结束".to_owned();
                        }
                        Err(error) => self.status = error.to_string(),
                    }
                }
            }
        }
    }

    fn show_finished(&mut self, ui: &mut egui::Ui) {
        let Some(match_state) = self.match_state.as_ref() else {
            return;
        };
        let game = match_state.game();
        if let Some(loser) = game.loser() {
            ui.heading("本局结束");
            ui.label(format!(
                "{} 拿到了最后的小王，判负。",
                game.players[loser].name
            ));
        }
        ui.label(format!("当前比分：{:?}", match_state.scores()));
    }

    fn show_match_progress(&mut self, ui: &mut egui::Ui) {
        let Some(match_state) = self.match_state.as_mut() else {
            return;
        };
        let mut restart = false;
        ui.label(format!("第 {} 局", match_state.round()));
        for (name, score) in match_state.names().iter().zip(match_state.scores()) {
            ui.label(format!("{}：{} 分", name, score));
        }
        if let Some(result) = match_state.history().last() {
            let ranking = result
                .finish_order
                .iter()
                .map(|player| match_state.names()[*player].as_str())
                .collect::<Vec<_>>()
                .join(" > ");
            ui.label(format!("第 {} 局出局排名：{}", result.round, ranking));
        }
        match match_state.phase() {
            MatchPhase::PlayoffPending { players } => {
                let names = players
                    .iter()
                    .map(|player| match_state.names()[*player].as_str())
                    .collect::<Vec<_>>()
                    .join("、");
                ui.label(format!("待加赛玩家：{names}"));
                if ui.button("开始加赛").clicked() {
                    match match_state.start_playoff() {
                        Ok(()) => self.status = "加赛已开始，请完成加赛整理".to_owned(),
                        Err(error) => self.status = error.to_string(),
                    }
                }
            }
            MatchPhase::Finished { winner } => {
                ui.label(format!("比赛结束，冠军：{}", match_state.names()[*winner]));
                if let Some(ranking) = match_state.playoff_ranking() {
                    ui.label(format!("加赛排名：{ranking:?}"));
                }
                restart = ui.button("重新开始").clicked();
            }
            MatchPhase::Playing | MatchPhase::Playoff { .. } => {}
        }
        if restart {
            self.restart();
        }
    }
}

impl eframe::App for YinhanApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(context, |ui| {
            ui.heading("53张牌鬼牌游戏");
            ui.label(&self.status);
            ui.separator();

            if self.match_state.is_none() {
                ui.label("四人本地热座玩法验证版");
                for (index, name) in self.names.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("玩家{}", index + 1));
                        ui.text_edit_singleline(name);
                    });
                }
                ui.horizontal(|ui| {
                    ui.label("随机种子");
                    ui.text_edit_singleline(&mut self.seed_text);
                });
                if ui.button("开始游戏").clicked() {
                    self.start_game();
                }
            } else {
                self.show_match_progress(ui);
                let phase = self.match_state.as_ref().map(|state| state.game().phase);
                let match_phase = self.match_state.as_ref().map(|state| state.phase().clone());
                match (phase, match_phase) {
                    (Some(GamePhase::Setup), _) => self.show_setup(ui),
                    (Some(GamePhase::Playing), _) => self.show_playing(ui),
                    (Some(GamePhase::Finished { .. }), Some(MatchPhase::Playing)) => {
                        if let Some(state) = self.match_state.as_mut() {
                            match state.settle_round() {
                                Ok(_) => self.status = "本局已结算，已进入下一局".to_owned(),
                                Err(error) => self.status = error.to_string(),
                            }
                        }
                    }
                    (Some(GamePhase::Finished { .. }), Some(MatchPhase::Playoff { .. })) => {
                        if ui.button("确认加赛结束并结算比赛").clicked() {
                            if let Some(state) = self.match_state.as_mut() {
                                match state.finish_playoff() {
                                    Ok(_) => self.status = "加赛排名已记录，比赛结束".to_owned(),
                                    Err(error) => self.status = error.to_string(),
                                }
                            }
                        }
                    }
                    (Some(GamePhase::Finished { .. }), _) => self.show_finished(ui),
                    _ => {}
                }
            }
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    eframe::run_native(
        "yinhan_play",
        eframe::NativeOptions::default(),
        Box::new(|creation_context| Ok(Box::new(YinhanApp::new(creation_context)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    wasm_bindgen_futures::spawn_local(async {
        let web_options = eframe::WebOptions::default();
        let canvas = eframe::web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id("yinhan_canvas"))
            .expect("yinhan_canvas element is missing")
            .dyn_into::<eframe::web_sys::HtmlCanvasElement>()
            .expect("yinhan_canvas is not a canvas element");
        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|creation_context| Ok(Box::new(YinhanApp::new(creation_context)))),
            )
            .await
            .expect("failed to start eframe web app");
    });
}
