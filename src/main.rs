use eframe::egui;
use yinhan_play::{GamePhase, GameState, PLAYER_COUNT, Pair, TurnPhase, XorShift64, find_pairs};

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::JsCast;

struct YinhanApp {
    names: [String; PLAYER_COUNT],
    seed_text: String,
    game: Option<GameState>,
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
            game: None,
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
        self.game = Some(GameState::new(self.names.clone(), seed));
        self.hand_visible = false;
        self.status = "游戏已开始，请按顺序完成开局整理".to_owned();
    }

    fn restart(&mut self) {
        self.game = None;
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
        let Some(game) = self.game.as_mut() else {
            return;
        };
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
        let Some(game) = self.game.as_mut() else {
            return;
        };
        if game.is_finished() {
            self.show_finished(ui);
            return;
        }
        let player = game.current_player;
        let player_name = game.players[player].name.clone();
        ui.heading(format!("{} 的回合", player_name));
        ui.label(format!("当前阶段：{:?}", game.turn_phase));

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
        ui.separator();

        match game.turn_phase {
            TurnPhase::AwaitingAction => {
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
                ui.label("抽牌后形成了对子，可以弃掉一组或保留。");
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
        let Some(game) = self.game.as_ref() else {
            return;
        };
        if let Some(loser) = game.loser() {
            ui.heading("游戏结束");
            ui.label(format!(
                "{} 拿到了最后的小王，判负。",
                game.players[loser].name
            ));
        }
        if ui.button("重新开始").clicked() {
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

            if self.game.is_none() {
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
                let phase = self.game.as_ref().map(|game| game.phase);
                match phase {
                    Some(GamePhase::Setup) => self.show_setup(ui),
                    Some(GamePhase::Playing) | Some(GamePhase::Finished { .. }) => {
                        self.show_playing(ui)
                    }
                    None => {}
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
