//! Focused KX27 workspace. The canvas, ink preview, PDF I/O and guided trap tools
//! are the same ones used by the editor; this layout keeps the artwork in view.
use egui::{Color32, Frame, Panel, RichText, Sense, Stroke, Ui, vec2};
use serde_json::{Value, json};
use vectorcraft_color::cms::PROCESS_PLATES;

use crate::{VectorcraftApp, canvas, chrome, state::zoom_label, theme::Tokens, widgets};

#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    pub open: bool,
    compact: Option<Pane>,
    document: Option<u64>,
    source: String,
    target: String,
    name: String,
    map_message: String,
    map_failed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pane {
    Inks,
    Trap,
}

pub(crate) fn set_open(app: &mut VectorcraftApp, open: bool) -> Result<Value, String> {
    if open && !app.ui.prepress.open {
        app.run("tool.select", json!({"tool":"hand"}))?;
        app.ui.home = None;
    }
    if app.ui.prepress.open != open {
        // Fit on the next canvas frame, after the panels change its available area.
        if let Some(view) = app.view_mut() {
            view.fitted = false;
        }
        app.canvas_rect = None;
    }
    app.ui.prepress.open = open;
    Ok(json!({"open":open}))
}

/// Explicit name-only edits or plate consolidation, through the shared undoable
/// swatch commands. Don't silently create a suffixed plate or convert to process.
pub(crate) fn map_spot(app: &mut VectorcraftApp, p: &Value) -> Result<Value, String> {
    let source = p["source"].as_str().ok_or("Elige la tinta plana de origen.")?;
    let doc = &app.session.active().ok_or("Abre un PDF para mapear tintas.")?.doc;
    let spot = |name| doc.swatch(name).is_some_and(|s| s.spot && !s.is_reserved() && s.paint.color().is_some());
    if !spot(source) {
        return Err("El origen debe ser una tinta plana del documento.".into());
    }
    let result = match (&p["newName"], &p["target"]) {
        (Value::String(name), Value::Null) => {
            let name = name.trim();
            if name.is_empty() || name.chars().count() > 96 || name.chars().any(char::is_control) {
                return Err("Escribe un nombre válido de hasta 96 caracteres.".into());
            }
            if name == source {
                return Err("Escribe un nombre distinto del actual.".into());
            }
            if PROCESS_PLATES.iter().any(|n| n.eq_ignore_ascii_case(name)) || doc.swatch(name).is_some() {
                return Err("Ese nombre ya existe. Para reunir dos tintas planas, usa Unificar con.".into());
            }
            app.run("swatch.edit", json!({"name":source,"newName":name}))?
        }
        (Value::Null, Value::String(target)) if target != source && spot(target) => app.run("swatch.merge", json!({"names":[target,source]}))?,
        _ => return Err("Elige un nombre nuevo o una tinta plana de destino distinta del origen.".into()),
    };
    // The previous visible-plate list can contain names that no longer exist.
    app.run("view.separationsPreview", json!({"on":false}))?;
    Ok(result)
}

fn run(app: &mut VectorcraftApp, id: &str, args: Value) {
    if let Err(e) = app.run(id, args)
        && e != "cancelled"
    {
        app.status(e);
    }
}

fn button(app: &mut VectorcraftApp, ui: &mut Ui, label: &str, id: &str, enabled: bool) {
    if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
        run(app, id, json!({}));
    }
}

/// The inherited selected fill blends with the dock; use the same accessible
/// foreground and accent pairing as the prepress action on active toggles.
pub(crate) fn mode_button(ui: &Ui, label: impl Into<String>, selected: bool) -> egui::Button<'static> {
    let label = label.into();
    if selected {
        egui::Button::new(RichText::new(label).color(Color32::WHITE)).selected(true).fill(Tokens::get(ui.ctx()).accent_strong)
    } else {
        egui::Button::new(label).selected(false)
    }
}

fn frame(fill: Color32, border: Color32) -> Frame {
    Frame::NONE.fill(fill).stroke(Stroke::new(1.0, border)).inner_margin(egui::Margin::symmetric(12, 9))
}

fn toolbar(app: &mut VectorcraftApp, ui: &mut Ui, wide: bool) {
    let doc_open = app.session.active().is_some();
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("KX27 Preprensa").size(16.0).strong());
        if ui.button("Volver al editor").clicked() {
            run(app, "prepress.workspace", json!({"open":false}));
        }
    });
    ui.add_space(5.0);
    ui.horizontal_wrapped(|ui| {
        button(app, ui, "Abrir PDF…", "prepress.openPdf", true);
        button(app, ui, "Exportar PDF…", "prepress.exportPdf", doc_open);
        ui.separator();
        if ui.add_enabled(doc_open, mode_button(ui, "Mano", app.session.tool_id() == "hand")).clicked() {
            run(app, "tool.select", json!({"tool":"hand"}));
        }
        if ui.add_enabled(doc_open, mode_button(ui, "Selección", app.session.tool_id() == "selection")).clicked() {
            run(app, "tool.select", json!({"tool":"selection"}));
        }
        button(app, ui, "Alejar", "view.zoomOut", doc_open);
        if let Some(view) = app.view() {
            ui.label(zoom_label(view.zoom));
        }
        button(app, ui, "Acercar", "view.zoomIn", doc_open);
        button(app, ui, "Ajustar", "view.fitArtboard", doc_open);
        button(app, ui, "Deshacer", "edit.undo", app.session.active().is_some_and(|d| !d.history.undo.is_empty()));
        if !wide {
            ui.separator();
            for (pane, label) in [(Pane::Inks, "Tintas"), (Pane::Trap, "Trapping")] {
                let selected = app.ui.prepress.compact == Some(pane);
                if ui.add(mode_button(ui, label, selected)).clicked() {
                    app.ui.prepress.compact = (!selected).then_some(pane);
                    if let Some(view) = app.view_mut() {
                        view.fitted = false;
                    }
                }
            }
        }
    });
}

fn ink_name(name: &str) -> &str {
    match name {
        "Cyan" => "Cian",
        "Magenta" => "Magenta",
        "Yellow" => "Amarillo",
        "Black" => "Negro",
        _ => name,
    }
}

fn inks(app: &mut VectorcraftApp, ui: &mut Ui) {
    ui.heading("Tintas");
    ui.add_space(5.0);
    let Some(st) = app.session.active() else {
        widgets::dim_label(ui, "Las separaciones aparecerán al abrir el PDF.");
        return;
    };
    let doc = st.doc.clone();
    let uid = st.uid;
    let view = vectorcraft_render::proof::view();
    let plates = vectorcraft_doc::inks::plates(&doc);
    let on = view.separations.is_some();
    egui::ScrollArea::vertical().id_salt("prepress-inks-scroll").auto_shrink([false, false]).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (label, separation) in [("Compuesta", false), ("Separaciones", true)] {
                if ui.add(mode_button(ui, label, on == separation)).clicked() {
                    run(app, "view.separationsPreview", json!({"on":separation}));
                }
            }
        });
        ui.add_space(7.0);
        for p in &plates {
            let mut visible = view.separations.as_ref().is_none_or(|names| names.contains(&p.name));
            ui.horizontal(|ui| {
                let [r, g, b] = p.rgb.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
                let (swatch, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                ui.painter().rect_filled(swatch, 2.0, Color32::from_rgb(r, g, b));
                let name = if p.spot { format!("{} · Plana", p.name) } else { ink_name(&p.name).into() };
                if ui.checkbox(&mut visible, name).on_hover_text("Alt al pulsar: aislar esta plancha").changed() {
                    let key = if ui.input(|i| i.modifiers.alt) { "only" } else { "toggle" };
                    run(app, "view.separationsPreview", json!({key:p.name}));
                }
            });
        }
        widgets::divider(ui);
        ui.strong("Mapeo de tintas");
        let spots: Vec<&str> = plates.iter().filter(|p| p.spot).map(|p| p.name.as_str()).collect();
        if spots.is_empty() {
            widgets::dim_label(ui, "No hay tintas planas en este documento.");
            return;
        }
        let state = &mut app.ui.prepress;
        if state.document != Some(uid) {
            state.document = Some(uid);
            state.source.clear();
            state.target.clear();
            state.name.clear();
            state.map_message.clear();
        }
        if !spots.contains(&state.source.as_str()) {
            state.source = spots.first().copied().unwrap_or_default().into();
        }
        ui.label("Tinta de origen");
        let old = state.source.clone();
        chooser(ui, "prepress-source", &mut state.source, spots.iter().copied());
        if state.source != old || !spots.contains(&state.target.as_str()) || state.target == state.source {
            state.target.clear();
        }
        let label = ui.label("Nombre nuevo");
        ui.add(egui::TextEdit::singleline(&mut state.name).desired_width(ui.available_width()).char_limit(96)).labelled_by(label.id);
        let rename = ui.add_enabled(!state.name.trim().is_empty(), egui::Button::new("Renombrar tinta")).clicked();
        let mut merge = false;
        if spots.len() > 1 {
            ui.add_space(6.0);
            ui.label("Unificar con");
            chooser(ui, "prepress-target", &mut state.target, spots.iter().copied().filter(|n| *n != state.source));
            merge = ui.add_enabled(!state.target.is_empty(), egui::Button::new("Unificar tintas")).clicked();
        }
        if rename || merge {
            let p = if rename { json!({"source":state.source,"newName":state.name}) } else { json!({"source":state.source,"target":state.target}) };
            match app.run("prepress.mapSpot", p) {
                Ok(result) => {
                    app.ui.prepress.source = result["name"].as_str().unwrap_or_default().into();
                    app.ui.prepress.name.clear();
                    app.ui.prepress.target.clear();
                    app.ui.prepress.map_failed = false;
                    app.ui.prepress.map_message = "Mapeo aplicado. Vuelve a detectar antes de generar franjas.".into();
                }
                Err(e) => {
                    app.ui.prepress.map_failed = true;
                    app.ui.prepress.map_message = e;
                }
            }
        }
        let state = &app.ui.prepress;
        if !state.map_message.is_empty() {
            if state.map_failed {
                ui.strong("No se pudo mapear la tinta");
            }
            ui.label(&state.map_message);
        }
        widgets::dim_label(ui, "Mapea antes del trapping. Las franjas ya incorporadas no se recalculan.");
    });
}

fn chooser<'a>(ui: &mut Ui, id: &str, value: &mut String, choices: impl Iterator<Item = &'a str>) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if value.is_empty() { "Elige una tinta" } else { value.as_str() })
        .width((ui.available_width() - 8.0).max(40.0))
        .show_ui(ui, |ui| {
            for name in choices {
                if ui.add(mode_button(ui, name, value == name)).clicked() {
                    *value = name.into();
                }
            }
        });
}

pub(crate) fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    if app.custom_titlebar {
        chrome::app_bar(app, ui);
    }
    let width = ui.available_width();
    let wide = width >= 920.0;
    Panel::top("prepress-tools").resizable(false).frame(frame(t.app_bar, t.border)).show(ui, |ui| toolbar(app, ui, wide));
    if !app.ui.status.is_empty() {
        let message = app.ui.status.chars().take(240).collect::<String>();
        Panel::bottom("prepress-status").resizable(false).frame(frame(t.app_bar, t.border)).show(ui, |ui| {
            ui.label(RichText::new(message).small()).on_hover_text(&app.ui.status);
        });
    }
    let pane_frame = frame(t.panel, t.border);
    if wide {
        Panel::left("prepress-inks").resizable(true).default_size(230.0).size_range(190.0..=310.0).frame(pane_frame).show(ui, |ui| inks(app, ui));
        Panel::right("prepress-trap")
            .resizable(true)
            .default_size(314.0)
            .size_range(270.0..=410.0)
            .frame(pane_frame)
            .show(ui, |ui| crate::panels::trapping::show_prepress(app, ui));
    } else if let Some(pane) = app.ui.prepress.compact {
        let panel = if width >= 620.0 {
            Panel::right("prepress-compact-side").exact_size(280.0)
        } else {
            let available = ui.available_height();
            let minimum = if pane == Pane::Trap { 205.0 } else { 95.0 };
            Panel::bottom("prepress-compact-bottom").exact_size((available * 0.43).clamp(minimum, 310.0).min(available))
        };
        panel.resizable(false).frame(pane_frame).show(ui, |ui| match pane {
            Pane::Inks => inks(app, ui),
            Pane::Trap => crate::panels::trapping::show_prepress(app, ui),
        });
    }
    egui::CentralPanel::default().frame(Frame::NONE.fill(t.pasteboard)).show(ui, |ui| {
        if app.session.active().is_some() {
            chrome::doc_tabs(app, ui);
            canvas::show(app, ui);
        } else {
            app.canvas_rect = None;
            let top = ui.available_height() * 0.28;
            ui.vertical_centered(|ui| {
                ui.add_space(top);
                ui.heading("Abre un PDF para comenzar");
                ui.label("Revisa las separaciones antes de mapear tintas y aplicar trapping.");
                ui.add_space(8.0);
                if widgets::primary_button(ui, "Elegir PDF…").clicked() {
                    run(app, "prepress.openPdf", json!({}));
                }
            });
        }
    });
}
