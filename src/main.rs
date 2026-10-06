#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use pulldown_cmark::{html, Options, Parser};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Default)]
struct Tab {
    id: u64,
    text: String,
    path: Option<PathBuf>,
    dirty: bool,
    cache: CommonMarkCache,
}
impl Tab {
    fn name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("無題.md")
            .to_string()
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Settings {
    dark: bool,
    recent: Vec<PathBuf>,
    autosave: bool,
}

struct MarkdownPocket {
    tabs: Vec<Tab>,
    active: usize,
    next_id: u64,
    show_editor: bool,
    show_preview: bool,
    wrap: bool,
    status: String,
    settings: Settings,
    data_dir: PathBuf,
    last_autosave: Instant,
}

impl MarkdownPocket {
    fn data_dir() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|x| x.join("MarkdownPocketData")))
            .unwrap_or_else(|| PathBuf::from("MarkdownPocketData"))
    }
    fn load() -> Self {
        let data_dir = Self::data_dir();
        let _ = fs::create_dir_all(data_dir.join("recovery"));
        let settings = fs::read_to_string(data_dir.join("settings.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Settings {
                dark: false,
                recent: vec![],
                autosave: true,
            });
        let mut s = Self {
            tabs: vec![],
            active: 0,
            next_id: 1,
            show_editor: true,
            show_preview: true,
            wrap: true,
            status: "準備完了".into(),
            settings,
            data_dir,
            last_autosave: Instant::now(),
        };
        s.new_tab();
        s
    }
    fn save_settings(&self) {
        if let Ok(s) = serde_json::to_string_pretty(&self.settings) {
            let _ = fs::write(self.data_dir.join("settings.json"), s);
        }
    }
    fn tab(&self) -> &Tab {
        &self.tabs[self.active]
    }
    fn tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active]
    }
    fn title(&self) -> String {
        format!(
            "{}{} - Markdown Pocket",
            self.tab().name(),
            if self.tab().dirty { " *" } else { "" }
        )
    }
    fn new_tab(&mut self) {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab {
            id,
            text: String::new(),
            path: None,
            dirty: false,
            cache: CommonMarkCache::default(),
        });
        self.active = self.tabs.len() - 1;
        self.status = "新しいタブ".into();
    }
    fn add_recent(&mut self, p: &Path) {
        self.settings.recent.retain(|x| x != p);
        self.settings.recent.insert(0, p.to_path_buf());
        self.settings.recent.truncate(12);
        self.save_settings();
    }
    fn open_path(&mut self, p: PathBuf) {
        if let Some(i) = self.tabs.iter().position(|t| t.path.as_ref() == Some(&p)) {
            self.active = i;
            return;
        }
        match fs::read_to_string(&p) {
            Ok(text) => {
                let id = self.next_id;
                self.next_id += 1;
                self.tabs.push(Tab {
                    id,
                    text,
                    path: Some(p.clone()),
                    dirty: false,
                    cache: CommonMarkCache::default(),
                });
                self.active = self.tabs.len() - 1;
                self.add_recent(&p);
                self.status = format!("開きました: {}", p.display());
            }
            Err(e) => self.status = format!("開けません: {e}"),
        }
    }
    fn open_dialog(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter(
                "Markdown / Text",
                &["md", "markdown", "mdown", "mkd", "txt"],
            )
            .pick_file()
        {
            self.open_path(p)
        }
    }
    fn save(&mut self) {
        if self.tab().path.is_none() {
            self.save_as();
            return;
        }
        let p = self.tab().path.clone().unwrap();
        match fs::write(&p, self.tab().text.as_bytes()) {
            Ok(_) => {
                self.tab_mut().dirty = false;
                self.add_recent(&p);
                self.status = format!("保存しました: {}", p.display());
            }
            Err(e) => self.status = format!("保存できません: {e}"),
        }
    }
    fn save_as(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("Markdown", &["md"])
            .set_file_name(&self.tab().name())
            .save_file()
        {
            self.tab_mut().path = Some(p);
            self.save();
        }
    }
    fn close_active(&mut self) {
        if self.tabs.len() == 1 {
            self.tabs[0] = Tab {
                id: self.next_id,
                text: String::new(),
                path: None,
                dirty: false,
                cache: CommonMarkCache::default(),
            };
            self.next_id += 1;
            return;
        }
        self.tabs.remove(self.active);
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
    }
    fn recovery_path(&self, id: u64) -> PathBuf {
        self.data_dir.join("recovery").join(format!("tab_{id}.md"))
    }
    fn autosave(&mut self) {
        if !self.settings.autosave || self.last_autosave.elapsed() < Duration::from_secs(8) {
            return;
        }
        self.last_autosave = Instant::now();
        for t in &self.tabs {
            if t.dirty {
                let header = format!(
                    "<!-- Markdown Pocket Recovery: {} -->\n",
                    t.path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "unsaved".into())
                );
                let _ = fs::write(self.recovery_path(t.id), format!("{header}{}", t.text));
            }
        }
    }
    fn export_html(&mut self) {
        let default = format!("{}.html", self.tab().name().trim_end_matches(".md"));
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("HTML", &["html"])
            .set_file_name(&default)
            .save_file()
        {
            let mut out = String::new();
            let parser = Parser::new_ext(&self.tab().text, Options::all());
            html::push_html(&mut out, parser);
            let doc = format!(
                r#"<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>{}</title><style>body{{font-family:Segoe UI,Meiryo,sans-serif;max-width:900px;margin:40px auto;padding:0 24px;line-height:1.7}}pre{{padding:14px;background:#f4f4f4;overflow:auto}}code{{font-family:Consolas,monospace}}img{{max-width:100%}}table{{border-collapse:collapse}}td,th{{border:1px solid #aaa;padding:6px 10px}}blockquote{{border-left:4px solid #aaa;padding-left:14px;color:#555}}</style></head><body>{}</body></html>"#,
                self.tab().name(),
                out
            );
            match fs::write(&p, doc) {
                Ok(_) => self.status = format!("HTML出力: {}", p.display()),
                Err(e) => self.status = format!("HTML出力失敗: {e}"),
            }
        }
    }
    fn print_pdf_hint(&mut self) {
        self.status =
            "PDF: HTML出力 → ブラウザで開く → Ctrl+P → Microsoft Print to PDF を選択".into();
    }
}

impl eframe::App for MarkdownPocket {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.settings.dark {
            ctx.set_visuals(egui::Visuals::dark())
        } else {
            ctx.set_visuals(egui::Visuals::light())
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.title()));
        self.autosave();
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            if let Some(p) = f.path {
                let ext = p
                    .extension()
                    .and_then(|x| x.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if ["md", "markdown", "mdown", "mkd", "txt"].contains(&ext.as_str()) {
                    self.open_path(p)
                }
            }
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
            self.save()
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::O)) {
            self.open_dialog()
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::N)) {
            self.new_tab()
        }

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("＋ 新規タブ").clicked() {
                    self.new_tab()
                }
                if ui.button("開く").clicked() {
                    self.open_dialog()
                }
                if ui.button("保存").clicked() {
                    self.save()
                }
                if ui.button("名前を付けて保存").clicked() {
                    self.save_as()
                }
                ui.menu_button("最近使ったファイル", |ui| {
                    let rec = self.settings.recent.clone();
                    if rec.is_empty() {
                        ui.label("履歴なし");
                    }
                    for p in rec {
                        if ui.button(p.display().to_string()).clicked() {
                            self.open_path(p);
                            ui.close_menu();
                        }
                    }
                });
                ui.menu_button("出力", |ui| {
                    if ui.button("HTMLとして保存").clicked() {
                        self.export_html();
                        ui.close_menu()
                    }
                    if ui.button("PDFにする方法").clicked() {
                        self.print_pdf_hint();
                        ui.close_menu()
                    }
                });
                ui.separator();
                ui.checkbox(&mut self.show_editor, "編集");
                ui.checkbox(&mut self.show_preview, "プレビュー");
                ui.checkbox(&mut self.wrap, "折返し");
                let mut dark = self.settings.dark;
                if ui.checkbox(&mut dark, "ダーク").changed() {
                    self.settings.dark = dark;
                    self.save_settings()
                }
                let mut auto = self.settings.autosave;
                if ui.checkbox(&mut auto, "リカバリー自動保存").changed() {
                    self.settings.autosave = auto;
                    self.save_settings()
                }
            });
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                let mut switch = None;
                let mut close = None;
                for (i, t) in self.tabs.iter().enumerate() {
                    let label = format!("{}{}", t.name(), if t.dirty { " ●" } else { "" });
                    if ui.selectable_label(i == self.active, label).clicked() {
                        switch = Some(i)
                    }
                    if i == self.active && ui.small_button("×").clicked() {
                        close = Some(i)
                    }
                }
                if let Some(i) = switch {
                    self.active = i
                }
                if close.is_some() {
                    self.close_active()
                }
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.separator();
                ui.label(format!(
                    "{}文字 / {}行",
                    self.tab().text.chars().count(),
                    self.tab().text.lines().count().max(1)
                ));
                if self.tab().dirty {
                    ui.separator();
                    ui.label("未保存");
                }
            })
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.show_editor && !self.show_preview {
                ui.centered_and_justified(|ui| {
                    ui.label("編集またはプレビューをONにしてください");
                });
                return;
            }
            if self.show_editor && self.show_preview {
                ui.columns(2, |cols| {
                    egui::ScrollArea::both()
                        .id_salt("edit")
                        .show(&mut cols[0], |ui| {
                            let wrap = self.wrap;
                            let t = self.tab_mut();
                            let mut e = egui::TextEdit::multiline(&mut t.text)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(30);
                            if !wrap {
                                e = e.code_editor()
                            }
                            if ui.add(e).changed() {
                                t.dirty = true
                            }
                        });
                    let t = self.tab_mut();
                    egui::ScrollArea::vertical()
                        .id_salt("preview")
                        .show(&mut cols[1], |ui| {
                            CommonMarkViewer::new().show(ui, &mut t.cache, &t.text);
                        });
                });
            } else if self.show_editor {
                let wrap = self.wrap;
                let t = self.tab_mut();
                egui::ScrollArea::both().show(ui, |ui| {
                    let mut e = egui::TextEdit::multiline(&mut t.text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(35);
                    if !wrap {
                        e = e.code_editor()
                    }
                    if ui.add(e).changed() {
                        t.dirty = true
                    }
                });
            } else {
                let t = self.tab_mut();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    CommonMarkViewer::new().show(ui, &mut t.cache, &t.text);
                });
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([760.0, 480.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Markdown Pocket",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(1.08);
            Ok(Box::new(MarkdownPocket::load()))
        }),
    )
}
