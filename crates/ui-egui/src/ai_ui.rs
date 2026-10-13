use std::sync::{Arc, Mutex};
use egui::{Color32, CornerRadius};
use crate::{LeftPanel, PdfCraftApp, theme::{self, Tokens}, widgets};
use crate::ai_ocr::{AiProvider, test_connection, query_ai_text};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AiScope {
    #[default]
    AllPages,
    CurrentPage,
    SelectedArea,
}

#[derive(Default)]
pub struct AiUiState {
    pub show_settings: bool,
    pub test_message: Option<Result<String, String>>,
    pub is_testing: bool,
    pub ask_input: String,
    pub response_text: Option<String>,
    pub is_working: bool,
    pub task_name: Option<String>,
    pub scope: AiScope,
    pub async_result: Option<Arc<Mutex<Option<Result<String, String>>>>>,
}

fn tr(s: &str) -> &str {
    if crate::i18n::current().code() == "th" {
        match s {
            "AI assistant" => "ผู้ช่วยเอไอ (AI)",
            "Ready" => "พร้อมใช้งาน",
            "Notice" => "ข้อแนะนำ",
            "Please configure an AI API Key" => "โปรดระบุหรือตั้งค่า API Key ของ AI",
            "Hide settings" => "ซ่อนการตั้งค่า",
            "Configure API Key & Model…" => "ตั้งค่า API Key และโมเดล AI…",
            "AI Provider & Model" => "ผู้ให้บริการและโมเดล AI",
            "Provider:" => "ผู้ให้บริการ:",
            "API Key" => "API Key",
            "Model" => "โมเดล",
            "Test connection" => "ทดสอบการเชื่อมต่อ",
            "Scope:" => "ขอบเขต:",
            "All pages" => "ทุกหน้า",
            "Current page" => "หน้าปัจจุบัน",
            "Select area" => "เลือกพื้นที่",
            "Drag a rectangle across text on the document to extract only that area with AI." => "ลากเมาส์วาดกรอบสี่เหลี่ยมคลุมข้อความบนหน้ากระดาษ เพื่อให้ AI อ่านและสกัดข้อความเฉพาะพื้นที่ที่เลือก",
            "Click to draw selection area" => "คลิกเพื่อเริ่มวาดเลือกพื้นที่",
            "Drawing mode active: drag on page" => "โหมดวาดพื้นที่เปิดอยู่: ลากเมาส์บนหน้าเอกสาร",
            "Extract text from images (AI OCR)" => "แปลงข้อความจากภาพด้วย AI (OCR)",
            "Transcribe Thai & English from images into accurate, copyable text layer" => "สกัดข้อความภาษาไทยและอังกฤษจากภาพ แปลงเป็นเลเยอร์ข้อความที่ค้นหาและคัดลอกได้",
            "Page" => "หน้า",
            "Cancel" => "ยกเลิก",
            "Draw & Extract Selected Area" => "วาดและสกัดข้อความจากพื้นที่ที่เลือก",
            "Start AI Text Extraction" => "เริ่มแปลงข้อความจากภาพด้วย AI",
            "Document Intelligence" => "ความสามารถอัจฉริยะของ AI",
            "Summarize" => "สรุปเนื้อหา",
            "Summarize document content in Thai" => "สรุปเนื้อหาเอกสารเป็นภาษาไทย",
            "Translate" => "แปลภาษา",
            "Translate document text to Thai" => "แปลข้อความเอกสารเป็นภาษาไทย",
            "Ask about this document" => "ถามคำถามเกี่ยวกับเอกสารนี้",
            "Type a question about this document…" => "พิมพ์คำถามเกี่ยวกับเอกสารนี้…",
            "Ask AI" => "ถาม AI",
            "AI Response" => "คำตอบจาก AI",
            "Copy" => "คัดลอก",
            "Clear" => "ล้าง",
            _ => crate::i18n::t(s),
        }
    } else {
        crate::i18n::t(s)
    }
}

fn notify_ai(app: &mut PdfCraftApp, s: &str) {
    if crate::i18n::current().code() == "th" {
        let msg = match s {
            "Text copied to clipboard!" => "คัดลอกข้อความลงคลิปบอร์ดแล้ว!",
            "Copied to clipboard" => "คัดลอกลงคลิปบอร์ดแล้ว",
            "Please enter your API Key in AI settings first" => "โปรดกรอก API Key ในการตั้งค่า AI ก่อนใช้งาน",
            "Drag a rectangle on the document to extract text" => "ลากกรอบสี่เหลี่ยมบนหน้าเอกสารเพื่อสกัดข้อความ",
            "Crosshair active: drag on page to select text" => "เปิดโหมดเป้าเล็งแล้ว: ลากบนหน้ากระดาษเพื่อเลือกข้อความ",
            "Open a document first" => "โปรดเปิดเอกสารก่อน",
            "Please enter a question" => "โปรดพิมพ์คำถาม",
            "Document cannot be read" => "ไม่สามารถอ่านเอกสารได้",
            "No text found. Run 'Extract text from images (AI OCR)' first." => "ไม่พบข้อความในเอกสาร โปรดเรียกใช้ 'แปลงข้อความจากภาพด้วย AI' ก่อน",
            _ => s,
        };
        app.notify(msg);
    } else {
        app.notify_tr(s);
    }
}

pub fn panel(app: &mut PdfCraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let (back, close) = crate::panels::panel_header(ui, t, tr("AI assistant"), true);
    if back {
        app.left = LeftPanel::AllTools;
        app.mode = crate::Mode::AllTools;
    }
    if close {
        app.left_open = false;
    }

    // Check async worker results from previous frames
    if let Some(res_arc) = app.ai_ui_state.async_result.clone() {
        if let Ok(mut lock) = res_arc.lock() {
            if let Some(res) = lock.take() {
                app.ai_ui_state.is_working = false;
                app.ai_ui_state.async_result = None;
                app.ai_ui_state.task_name = None;
                match res {
                    Ok(text) => {
                        #[cfg(not(target_arch = "wasm32"))]
                        if let Ok(mut cb) = arboard::Clipboard::new() {
                            let _ = cb.set_text(text.clone());
                        }
                        notify_ai(app, "Text copied to clipboard!");
                        app.ai_ui_state.response_text = Some(text);
                    }
                    Err(err) => {
                        app.notify_fmt("AI error: {err}", &[("err", &err)]);
                    }
                }
            }
        }
    }

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        // --- 1. Connection Status Card ---
        let has_key = !app.ai_config.api_key.trim().is_empty() || app.ai_config.provider == AiProvider::Custom;
        egui::Frame::new()
            .fill(t.hover)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let dot_col = if has_key { Color32::from_rgb(0x22, 0xC5, 0x5E) } else { Color32::from_rgb(0xEA, 0x58, 0x0C) };
                    ui.label(egui::RichText::new("●").color(dot_col).font(theme::semibold(14.0)));
                    let status_text = if has_key {
                        format!("{}: {} ({})", tr("Ready"), app.ai_config.provider.display_name(), app.ai_config.model)
                    } else {
                        format!("{}: {}", tr("Notice"), tr("Please configure an AI API Key"))
                    };
                    ui.label(egui::RichText::new(status_text).font(theme::medium(12.0)).color(t.text));
                });

                ui.add_space(4.0);
                let btn_text = if app.ai_ui_state.show_settings {
                    tr("Hide settings")
                } else {
                    tr("Configure API Key & Model…")
                };
                if widgets::pill_button(ui, btn_text, app.ai_ui_state.show_settings).clicked() {
                    app.ai_ui_state.show_settings = !app.ai_ui_state.show_settings;
                    app.ai_ui_state.test_message = None;
                }
            });

        // --- 2. Collapsible Settings Section ---
        if app.ai_ui_state.show_settings {
            ui.add_space(6.0);
            egui::Frame::new()
                .fill(t.card)
                .stroke(egui::Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new(tr("AI Provider & Model")).font(theme::semibold(13.0)).color(t.text));
                    ui.add_space(6.0);

                    // Provider selector
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(tr("Provider:")).color(t.text_muted));
                        let current_name = app.ai_config.provider.display_name();
                        egui::ComboBox::from_id_salt("ai-panel-provider")
                            .selected_text(current_name)
                            .width(180.0)
                            .show_ui(ui, |ui| {
                                for p in AiProvider::ALL {
                                    if ui.selectable_label(app.ai_config.provider == p, p.display_name()).clicked() {
                                        app.ai_config.provider = p;
                                        app.ai_config.model = p.default_model().to_string();
                                        app.ai_ui_state.test_message = None;
                                    }
                                }
                            });
                    });

                    ui.add_space(6.0);
                    // API Key input
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{}:", tr("API Key"))).color(t.text_muted));
                        let w = (ui.available_width() - 8.0).max(120.0);
                        ui.add(egui::TextEdit::singleline(&mut app.ai_config.api_key).password(true).desired_width(w));
                    });

                    ui.add_space(6.0);
                    // Model input / presets
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{}:", tr("Model"))).color(t.text_muted));
                        let presets = app.ai_config.provider.presets();
                        let current_model = app.ai_config.model.clone();
                        egui::ComboBox::from_id_salt("ai-panel-model")
                            .selected_text(&current_model)
                            .width(180.0)
                            .show_ui(ui, |ui| {
                                for &preset in presets {
                                    if ui.selectable_label(app.ai_config.model == preset, preset).clicked() {
                                        app.ai_config.model = preset.to_string();
                                    }
                                }
                            });
                    });

                    if app.ai_config.provider == AiProvider::Custom {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Custom Endpoint:").color(t.text_muted));
                        ui.add(egui::TextEdit::singleline(&mut app.ai_config.custom_endpoint).desired_width(ui.available_width()));
                    }

                    ui.add_space(8.0);
                    // Test connection button
                    ui.horizontal(|ui| {
                        if ui.button(tr("Test connection")).clicked() {
                            let res = test_connection(&app.ai_config);
                            app.ai_ui_state.test_message = Some(res);
                        }
                    });

                    if let Some(msg) = &app.ai_ui_state.test_message {
                        ui.add_space(4.0);
                        match msg {
                            Ok(ok_text) => {
                                ui.label(egui::RichText::new(ok_text).color(Color32::from_rgb(0x16, 0xA3, 0x4A)).font(theme::regular(11.5)));
                            }
                            Err(err_text) => {
                                ui.label(egui::RichText::new(err_text).color(Color32::from_rgb(0xDC, 0x26, 0x26)).font(theme::regular(11.5)));
                            }
                        }
                    }
                });
        }

        ui.add_space(10.0);

        // Scope toggle: All pages vs Current page vs Draw / Select area
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(tr("Scope:")).color(t.text_muted).font(theme::medium(12.0)));
            if ui.selectable_label(app.ai_ui_state.scope == AiScope::AllPages, tr("All pages")).clicked() {
                app.ai_ui_state.scope = AiScope::AllPages;
                if app.quick_tool == crate::QuickTool::AiSelect {
                    app.quick_tool = crate::QuickTool::Select;
                }
            }
            if ui.selectable_label(app.ai_ui_state.scope == AiScope::CurrentPage, tr("Current page")).clicked() {
                app.ai_ui_state.scope = AiScope::CurrentPage;
                if app.quick_tool == crate::QuickTool::AiSelect {
                    app.quick_tool = crate::QuickTool::Select;
                }
            }
            if ui.selectable_label(app.ai_ui_state.scope == AiScope::SelectedArea, format!("✂ {}", tr("Select area"))).clicked() {
                app.ai_ui_state.scope = AiScope::SelectedArea;
                app.quick_tool = crate::QuickTool::AiSelect;
                notify_ai(app, "Drag a rectangle on the document to extract text");
            }
        });

        if app.ai_ui_state.scope == AiScope::SelectedArea {
            ui.add_space(4.0);
            egui::Frame::new()
                .fill(t.accent_soft)
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(tr("Drag a rectangle across text on the document to extract only that area with AI."))
                            .font(theme::regular(11.5))
                            .color(t.text),
                    );
                    if app.quick_tool != crate::QuickTool::AiSelect {
                        ui.add_space(4.0);
                        if ui.button(format!("✏ {}", tr("Click to draw selection area"))).clicked() {
                            app.quick_tool = crate::QuickTool::AiSelect;
                            notify_ai(app, "Crosshair active: drag on page to select text");
                        }
                    } else {
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(format!("● {}", tr("Drawing mode active: drag on page")))
                                .color(t.accent_text)
                                .font(theme::semibold(11.5)),
                        );
                    }
                });
        }

        ui.add_space(10.0);

        // --- 3. Feature 1: AI Vision OCR / Extract Text from Images ---
        widgets::section_title(ui, tr("Extract text from images (AI OCR)"));
        ui.label(
            egui::RichText::new(tr("Transcribe Thai & English from images into accurate, copyable text layer"))
                .color(t.text_faint)
                .font(theme::regular(11.5)),
        );
        ui.add_space(6.0);

        // If OCR is currently running: show progress bar
        if let Some(run) = &app.ocr_run {
            if let Ok(mut p) = run.progress.lock() {
                let frac = if p.total > 0 { p.done as f32 / p.total as f32 } else { 0.0 };
                ui.add(egui::ProgressBar::new(frac).text(format!("{} {} / {}", tr("Page"), p.done, p.total)));
                ui.add_space(4.0);
                if ui.button(tr("Cancel")).clicked() {
                    p.cancel = true;
                }
            }
        } else {
            let btn_label = if app.ai_ui_state.scope == AiScope::SelectedArea {
                format!("✂ {}", tr("Draw & Extract Selected Area"))
            } else {
                format!("🚀 {}", tr("Start AI Text Extraction"))
            };
            let ocr_btn = ui.add(
                egui::Button::new(
                    egui::RichText::new(btn_label)
                        .font(theme::semibold(13.0))
                        .color(Color32::WHITE),
                )
                .fill(t.accent)
                .corner_radius(CornerRadius::same(6)),
            );
            if ocr_btn.clicked() {
                if app.active_ids().is_some() {
                    if app.ai_config.api_key.trim().is_empty() && app.ai_config.provider != AiProvider::Custom {
                        app.ai_ui_state.show_settings = true;
                        notify_ai(app, "Please enter your API Key in AI settings first");
                    } else if app.ai_ui_state.scope == AiScope::SelectedArea {
                        app.quick_tool = crate::QuickTool::AiSelect;
                        notify_ai(app, "Drag a rectangle on the document to extract text");
                    } else {
                        app.ai_config.enabled = true;
                        app.ocr_draft.use_ai = true;
                        if app.ai_ui_state.scope == AiScope::AllPages {
                            app.ocr_draft.pages = crate::ocr_ui::OcrPages::All;
                        } else {
                            app.ocr_draft.pages = crate::ocr_ui::OcrPages::Current;
                        }
                        app.start_ocr();
                    }
                } else {
                    notify_ai(app, "Open a document first");
                }
            }
        }

        ui.add_space(14.0);

        // --- 4. Feature 2: Summarize Document ---
        widgets::section_title(ui, tr("Document Intelligence"));
        
        ui.horizontal(|ui| {
            if ui.button(format!("✨ {}", tr("Summarize"))).on_hover_text(tr("Summarize document content in Thai")).clicked() {
                trigger_ai_action(app, "Summarize", "You are an intelligent document assistant. Summarize the following document clearly, accurately, and concisely in Thai.");
            }

            if ui.button(format!("🌐 {}", tr("Translate"))).on_hover_text(tr("Translate document text to Thai")).clicked() {
                trigger_ai_action(app, "Translate", "You are an expert translator. Translate the key content of this document into natural, professional Thai.");
            }
        });

        ui.add_space(8.0);

        // --- 5. Feature 3: Ask about this document ---
        ui.label(egui::RichText::new(tr("Ask about this document")).font(theme::medium(12.5)).color(t.text));
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::multiline(&mut app.ai_ui_state.ask_input)
                .hint_text(tr("Type a question about this document…"))
                .desired_rows(2)
                .desired_width(ui.available_width()),
        );
        ui.add_space(4.0);
        if ui.button(format!("💬 {}", tr("Ask AI"))).clicked() {
            let question = app.ai_ui_state.ask_input.trim().to_string();
            if question.is_empty() {
                notify_ai(app, "Please enter a question");
            } else {
                let sys_prompt = format!("You are a knowledgeable assistant. Answer the user's question accurately based ONLY on the provided document text in Thai.\nQuestion: {question}");
                trigger_ai_action(app, "Ask", &sys_prompt);
            }
        }

        ui.add_space(14.0);

        // --- 6. Results display ---
        if app.ai_ui_state.is_working {
            let task = app.ai_ui_state.task_name.as_deref().unwrap_or("Processing");
            egui::Frame::new()
                .fill(t.accent_soft)
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(egui::RichText::new(format!("AI {task}…")).color(t.text).font(theme::medium(12.5)));
                    });
                });
        }

        let resp_opt = app.ai_ui_state.response_text.clone();
        if let Some(resp) = resp_opt {
            widgets::section_title(ui, tr("AI Response"));
            let mut do_copy = false;
            let mut do_clear = false;
            egui::Frame::new()
                .fill(t.card)
                .stroke(egui::Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new(&resp).font(theme::regular(12.5)).color(t.text));

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(format!("📋 {}", tr("Copy"))).clicked() {
                            ui.ctx().copy_text(resp.clone());
                            do_copy = true;
                        }
                        if ui.button(tr("Clear")).clicked() {
                            do_clear = true;
                        }
                    });
                });
            if do_copy {
                notify_ai(app, "Copied to clipboard");
            }
            if do_clear {
                app.ai_ui_state.response_text = None;
            }
        }
    });
}

fn trigger_ai_action(app: &mut PdfCraftApp, action_name: &'static str, system_prompt: &str) {
    if app.ai_config.api_key.trim().is_empty() && app.ai_config.provider != AiProvider::Custom {
        app.ai_ui_state.show_settings = true;
        notify_ai(app, "Please enter your API Key in AI settings first");
        return;
    }

    let Some((vi, id)) = app.active_ids() else {
        notify_ai(app, "Open a document first");
        return;
    };

    let Some(doc) = app.session.get(id) else {
        notify_ai(app, "Document cannot be read");
        return;
    };

    let total_pages = doc.info.pages.len();
    let pages: Vec<usize> = if app.ai_ui_state.scope == AiScope::AllPages {
        (0..total_pages).collect()
    } else {
        vec![app.views[vi].current.min(total_pages.saturating_sub(1))]
    };

    let mut exporter = pdfcraft_engine::export::Exporter::new(doc);
    let doc_text = match exporter.text_of(&pages) {
        Ok(t) => t,
        Err(e) => {
            app.notify_fmt("Could not extract text: {e}", &[("e", &e)]);
            return;
        }
    };

    if doc_text.trim().is_empty() {
        notify_ai(app, "No text found. Run 'Extract text from images (AI OCR)' first.");
        return;
    }

    // Limit text to ~24000 characters to prevent token overflow
    let truncated_text = if doc_text.len() > 24000 {
        let mut s = doc_text.chars().take(24000).collect::<String>();
        s.push_str("\n\n[Document truncated for length...]");
        s
    } else {
        doc_text
    };

    let cfg = app.ai_config.clone();
    let sys = system_prompt.to_string();
    let res_arc = Arc::new(Mutex::new(None));
    app.ai_ui_state.async_result = Some(res_arc.clone());
    app.ai_ui_state.is_working = true;
    app.ai_ui_state.task_name = Some(action_name.to_string());

    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || {
        let res = query_ai_text(&cfg, &sys, &truncated_text);
        if let Ok(mut lock) = res_arc.lock() {
            *lock = Some(res);
        }
    });
}
