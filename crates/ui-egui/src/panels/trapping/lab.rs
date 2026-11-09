//! Large, modeless development workflow over the same palette caches and commands.
//! Stages guide the operator; none of them calculate geometry during ordinary frames.
use super::*;
use std::sync::Weak;

// Preserve the six development-stage IDs used by diagnostics and the floating window.
const ADJUST: usize = 6;
const EXPORT_PDF: usize = 7;
const PREPRESS_FLOW: [(usize, &str); 8] = [
    (0, "Detectar"),
    (1, "Zona y par"),
    (2, "Verificar curvas"),
    (ADJUST, "Ajustar franja"),
    (3, "Previsualizar"),
    (4, "Incorporar"),
    (5, "Diagnóstico"),
    (EXPORT_PDF, "Exportar PDF"),
];

const STEPS: [&str; 6] = ["Detectar", "Zona y par", "Verificar curvas", "Previsualizar", "Incorporar", "Diagnóstico"];
#[derive(Clone, Debug, Default)]
pub(super) struct State {
    pub step: usize,
    pub pair: Option<usize>,
    pub receipt: Option<Receipt>,
    pub omissions: Vec<Omission>,
    pub review_region: Option<[f64; 4]>,
    adjusted: Option<Weak<CurveCheck>>,
    drawing_scope: bool,
    scope_start: Option<egui::Pos2>,
}
impl State {
    fn cancel_scope(&mut self) {
        self.drawing_scope = false;
        self.scope_start = None;
    }

    pub(super) fn clear_scope(&mut self) {
        self.cancel_scope();
        self.review_region = None;
    }
}
#[derive(Clone, Debug)]
pub(super) struct Omission {
    boundary: Weak<BoundaryCapture>,
    zone: usize,
    pair: usize,
}
fn omitted(settings: &Settings, zone: usize, pair: usize) -> bool {
    settings
        .boundaries
        .as_ref()
        .is_some_and(|capture| settings.lab.omissions.iter().any(|o| o.zone == zone && o.pair == pair && o.boundary.ptr_eq(&Arc::downgrade(capture))))
}
fn current_omitted(settings: &Settings) -> bool {
    settings.boundary_selected.zip(settings.lab.pair).is_some_and(|(z, p)| omitted(settings, z, p))
}
fn current_capture(app: &VectorcraftApp, settings: &Settings) -> Result<Arc<BoundaryCapture>, String> {
    let c = settings.boundaries.clone().ok_or("Detecta las fronteras y elige un par primero.")?;
    let artboard = app.session.active_index().and_then(|i| app.views.get(i)).map_or(0, |v| v.artboard);
    if app.session.active().is_none_or(|st| Some(st.uid) != settings.document || !Arc::ptr_eq(&c.document, &st.doc))
        || c.colors != vectorcraft_color::cms::active_settings()
        || c.artboard != artboard
        || c.dpi != settings.boundary_dpi
        || c.contrast != settings.boundary_contrast
    {
        return Err("El análisis cambió. Detecta las fronteras nuevamente antes de decidir.".into());
    }
    Ok(c)
}
/// A review decision for this capture only: never changes artwork or undo history.
pub fn set_omission(app: &mut VectorcraftApp, params: &Value) -> Result<Value, String> {
    let omit = match params.get("omit") {
        None => true,
        Some(Value::Bool(v)) => *v,
        _ => return Err("omit debe ser verdadero o falso.".into()),
    };
    let mut settings = app.ui.trapping.clone();
    let c = current_capture(app, &settings)?;
    let zone = settings.boundary_selected.ok_or("Elige una zona.")?;
    let index = settings.lab.pair.ok_or("Elige un par concreto.")?;
    let ids = pair(&settings).ok_or("Elige un par válido.")?["ids"].clone();
    settings.lab.omissions.retain(|o| o.boundary.ptr_eq(&Arc::downgrade(&c)));
    let was_omitted = omitted(&settings, zone, index);
    if omit && !was_omitted {
        if settings.lab.omissions.len() >= 512 {
            return Err("Este análisis alcanzó 512 omisiones. Exporta su diagnóstico antes de iniciar otro análisis.".into());
        }
        settings.lab.omissions.push(Omission { boundary: Arc::downgrade(&c), zone, pair: index });
    } else if !omit {
        settings.lab.omissions.retain(|o| o.zone != zone || o.pair != index);
    }
    settings.curves = None;
    settings.preview = None;
    settings.lab.step = 1;
    settings.failed = false;
    settings.export_message.clear();
    settings.export_failed = false;
    settings.message = if omit {
        "Par omitido por decisión del operador. No se creó ninguna franja. Puedes revisar otro par o volver a evaluarlo.".into()
    } else {
        "Omisión retirada. Puedes verificar y previsualizar este par nuevamente.".into()
    };
    app.ui.trapping = settings;
    Ok(json!({"status":if omit {"omitted"} else {"pending"},"zone":zone+1,"pair":index+1,"ids":ids,"documentModified":false}))
}
pub fn next_pair(app: &mut VectorcraftApp, _params: &Value) -> Result<Value, String> {
    let mut settings = app.ui.trapping.clone();
    let c = current_capture(app, &settings)?;
    let pairs: Vec<_> = c.report["zones"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .flat_map(|(z, zone)| (0..zone["attribution"]["pairs"].as_array().map_or(0, Vec::len)).map(move |p| (z, p)))
        .collect();
    let current = settings.boundary_selected.zip(settings.lab.pair);
    let start = current.and_then(|v| pairs.iter().position(|p| *p == v)).map_or(0, |i| i + 1);
    let next = pairs.iter().cycle().skip(start).take(pairs.len()).copied().find(|(z, p)| Some((*z, *p)) != current && !omitted(&settings, *z, *p));
    if let Some((z, p)) = next {
        select_zone(app, &mut settings, &c, z);
        select_pair(app, &mut settings, p);
        settings.lab.step = 1;
    } else {
        settings.lab.step = 5;
        settings.failed = false;
        settings.message =
            "No quedan otros pares atribuibles sin omitir en este análisis. Puedes exportar el diagnóstico o volver a Zona y par.".into();
    }
    app.ui.trapping = settings;
    Ok(json!({"next":next.map(|(z,p)|json!({"zone":z+1,"pair":p+1}))}))
}
fn decision_action(app: &mut VectorcraftApp, settings: &mut Settings, command: &str, params: Value) {
    app.ui.trapping = settings.clone();
    let result = app.run(command, params);
    *settings = app.ui.trapping.clone();
    if let Err(e) = result {
        settings.failed = true;
        settings.message = e;
    }
}

#[derive(Clone, Debug)]
pub(super) struct Receipt {
    pub uid: u64,
    pub document: Weak<Document>,
    pub report: Value,
}

fn pair(settings: &Settings) -> Option<&Value> {
    let capture = settings.boundaries.as_ref()?;
    let zone = capture.report["zones"].as_array()?.get(settings.boundary_selected?)?;
    zone["attribution"]["pairs"].as_array()?.get(settings.lab.pair?)
}
fn curves_current(settings: &Settings) -> bool {
    settings.curves.as_ref().is_some_and(|c| {
        settings.boundaries.as_ref().is_some_and(|b| Arc::ptr_eq(&c.boundary, b))
            && settings.boundary_selected == Some(c.zone)
            && settings.lab.pair == Some(c.pair)
    })
}
fn preview_current(settings: &Settings) -> bool {
    curves_current(settings)
        && settings.preview.as_ref().is_some_and(|p| {
            settings.curves.as_ref().is_some_and(|c| Arc::ptr_eq(c, &p.check))
                && p.width_mm == settings.preview_width_mm
                && p.reverse == settings.preview_reverse
        })
}
fn receipt_current(app: &VectorcraftApp, settings: &Settings) -> bool {
    settings
        .lab
        .receipt
        .as_ref()
        .is_some_and(|r| app.session.active().is_some_and(|st| st.uid == r.uid && r.document.upgrade().is_some_and(|doc| Arc::ptr_eq(&doc, &st.doc))))
}
fn allowed(settings: &Settings, step: usize) -> bool {
    match step {
        0 | 5 | EXPORT_PDF => true,
        1 => settings.boundaries.is_some(),
        2 => pair(settings).is_some() && !current_omitted(settings),
        3 | 4 | ADJUST => {
            !current_omitted(settings)
                && curves_current(settings)
                && settings.curves.as_ref().is_some_and(|c| c.report["visibleSamples"].as_u64().unwrap_or(0) > 0)
        }
        _ => false,
    }
}
fn completed(settings: &Settings, step: usize) -> bool {
    match step {
        0 => settings.boundaries.is_some(),
        1 => pair(settings).is_some(),
        2 => curves_current(settings),
        3 => preview_current(settings),
        4 => settings.lab.receipt.is_some(),
        5 => !settings.export_message.is_empty() && !settings.export_failed,
        ADJUST => {
            settings.lab.adjusted.as_ref().is_some_and(|check| settings.curves.as_ref().is_some_and(|current| check.ptr_eq(&Arc::downgrade(current))))
                || preview_current(settings)
        }
        _ => false,
    }
}
fn select_zone(app: &mut VectorcraftApp, settings: &mut Settings, c: &Arc<BoundaryCapture>, index: usize) {
    choose_zone(app, settings, c, index);
    let pairs = c.report["zones"].as_array().and_then(|v| v.get(index)).and_then(|z| z["attribution"]["pairs"].as_array());
    settings.lab.pair = pairs.filter(|p| p.len() == 1).map(|_| 0);
}
fn select_pair(app: &mut VectorcraftApp, settings: &mut Settings, index: usize) {
    settings.lab.clear_scope();
    settings.lab.pair = Some(index);
    settings.curves = None;
    settings.preview = None;
    if let Some(p) = pair(settings) {
        match app.run("select.set", json!({"ids":p["ids"]})) {
            Ok(_) => {
                settings.failed = false;
                settings.message = "Par elegido. Los originales están resaltados en el documento.".into();
            }
            Err(e) => {
                settings.failed = true;
                settings.message = e;
            }
        }
    }
}
fn label_ids(settings: &Settings, ids: &Value) -> String {
    ids.as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| settings.boundaries.as_ref()?.source_labels.get(&v.as_u64()?).cloned())
        .collect::<Vec<_>>()
        .join(" ↔ ")
}
fn context(ui: &mut Ui, settings: &Settings) {
    if let Some(index) = settings.boundary_selected {
        let number = settings.lab.pair.map_or_else(|| "Elige un par".into(), |p| format!("Par {}", p + 1));
        ui.strong(format!("Zona {} · {number}", index + 1));
        if let Some(p) = pair(settings) {
            ui.label(label_ids(settings, &p["ids"]));
        }
        if let Some(region) = settings.lab.review_region {
            ui.label(scope_size(region));
        }
    }
}

/// Called by the app's normal frame, independently of the dock palette's visibility.
pub fn show_window(app: &mut VectorcraftApp, ctx: &egui::Context) {
    if !app.ui.trapping.development_window {
        return;
    }
    let mut settings = app.ui.trapping.clone();
    let mut open = true;
    let viewport = ctx.content_rect();
    let size = egui::vec2((viewport.width() - 32.0).clamp(1.0, 1100.0), (viewport.height() - 48.0).clamp(1.0, 740.0));
    egui::Window::new("Trapping · Desarrollo")
        .id(egui::Id::new("kx27-development-window"))
        .open(&mut open)
        .default_pos(viewport.min + egui::vec2(12.0, 24.0))
        .default_size(size)
        .max_size(size)
        .min_size(egui::vec2(620.0_f32.min(size.x), 440.0_f32.min(size.y)))
        .constrain_to(viewport.shrink(8.0))
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| show_contents(app, ui, &mut settings));
    settings.development_window = open;
    app.ui.trapping = settings;
}

fn refresh(app: &mut VectorcraftApp, ui: &Ui, settings: &mut Settings, docked: bool) {
    sync_document(app, ui, settings);
    invalidate_boundaries(app, settings);
    if settings.lab.receipt.is_some() && !receipt_current(app, settings) {
        settings.lab.receipt = None;
    }
    if settings.lab.pair.is_none()
        && let Some(check) = settings.curves.as_ref()
    {
        settings.lab.pair = Some(check.pair);
    }
    if settings.preview.is_some() && !preview_current(settings) {
        settings.preview = None;
    }
    settings.lab.omissions.retain(|o| settings.boundaries.as_ref().is_some_and(|c| o.boundary.ptr_eq(&Arc::downgrade(c))));
    if !docked {
        settings.lab.step = match settings.lab.step {
            ADJUST => 3,
            EXPORT_PDF => 5,
            step => step.min(5),
        };
    }
    if !allowed(settings, settings.lab.step) {
        settings.lab.step = if settings.boundaries.is_none() {
            0
        } else if pair(settings).is_none() {
            1
        } else {
            2
        };
    }
    if settings.lab.step != 1 {
        settings.lab.cancel_scope();
    }
}

fn embedded_step_allowed(settings: &Settings, step: usize) -> bool {
    allowed(settings, step) && (step != 4 || preview_current(settings))
}

fn flow_index(step: usize) -> usize {
    PREPRESS_FLOW.iter().position(|&(id, _)| id == step).unwrap_or(0)
}

fn flow_done(settings: &Settings, step: usize) -> bool {
    completed(settings, step) || (flow_index(step) < 6 && settings.lab.receipt.is_some())
}

fn flow_arrow(ui: &Ui, from: egui::Pos2, to: egui::Pos2, color: egui::Color32) {
    let axis = (to - from).normalized();
    let across = egui::vec2(-axis.y, axis.x);
    let end = to - axis * 10.0;
    let start = from + axis * 10.0;
    if (end - start).dot(axis) < 2.0 {
        return;
    }
    let stroke = egui::Stroke::new(1.25, color);
    ui.painter().line_segment([start, end], stroke);
    let tail = end - axis * 3.0;
    ui.painter().line_segment([tail + across * 2.0, end], stroke);
    ui.painter().line_segment([end, tail - across * 2.0], stroke);
}

fn flow_node(ui: &mut Ui, settings: &mut Settings, index: usize, size: egui::Vec2, vertical: bool, previous: Option<egui::Pos2>) -> egui::Pos2 {
    let (id, name) = PREPRESS_FLOW[index];
    let active = settings.lab.step == id;
    let done = flow_done(settings, id);
    let enabled = embedded_step_allowed(settings, id);
    let status = if active {
        "Actual"
    } else if done {
        "Listo"
    } else if enabled {
        "Disponible"
    } else {
        "Pendiente"
    };
    let label = format!("{}. {name} · {status}", index + 1);
    let (rect, response) = ui.add_enabled_ui(enabled, |ui| ui.allocate_exact_size(size, egui::Sense::click())).inner;
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &label));
    if response.clicked() {
        settings.lab.step = id;
    }
    let tokens = Tokens::get(ui.ctx());
    let center = if vertical { egui::pos2(rect.left() + 13.0, rect.center().y) } else { rect.center() };
    if let Some(previous) = previous {
        flow_arrow(ui, previous, center, tokens.text_dim);
    }
    let painter = ui.painter();
    if active && vertical {
        painter.rect_filled(egui::Rect::from_min_max(rect.min + egui::vec2(26.0, 0.0), rect.max), 3.0, tokens.accent_strong);
    } else if response.hovered() && enabled {
        painter.rect_filled(rect, 3.0, ui.visuals().widgets.hovered.bg_fill);
    }
    if response.has_focus() {
        painter.rect_stroke(rect.shrink(1.0), 4.0, egui::Stroke::new(1.5, tokens.text), egui::StrokeKind::Inside);
    }
    if active || done {
        painter.circle_filled(center, 9.5, tokens.accent_strong);
        if active {
            painter.circle_stroke(center, 10.0, egui::Stroke::new(1.0, egui::Color32::WHITE));
        }
    } else {
        painter.circle_stroke(center, 9.0, egui::Stroke::new(1.0, tokens.text_dim));
    }
    let font = egui::TextStyle::Body.resolve(ui.style());
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        (index + 1).to_string(),
        font.clone(),
        if active || done { egui::Color32::WHITE } else { tokens.text },
    );
    if vertical {
        let color = if active { egui::Color32::WHITE } else { tokens.text };
        painter.text(egui::pos2(rect.left() + 34.0, center.y), egui::Align2::LEFT_CENTER, name, font.clone(), color);
        painter.text(
            egui::pos2(rect.right() - 7.0, center.y),
            egui::Align2::RIGHT_CENTER,
            status,
            egui::TextStyle::Small.resolve(ui.style()),
            if active { color } else { tokens.text_dim },
        );
    }
    response.on_hover_text(label);
    center
}

fn embedded_steps(ui: &mut Ui, settings: &mut Settings) {
    let vertical = ui.available_height() >= 490.0 && ui.available_width() >= 220.0;
    ui.scope(|ui| {
        let mut previous = None;
        if vertical {
            ui.spacing_mut().item_spacing.y = 4.0;
            let width = ui.available_width();
            for i in 0..PREPRESS_FLOW.len() {
                previous = Some(flow_node(ui, settings, i, egui::vec2(width, 24.0), true, previous));
            }
        } else {
            ui.spacing_mut().item_spacing.x = 4.0;
            let columns = if ui.available_width() >= 218.0 { 8 } else { 4 };
            let width = ((ui.available_width() - (columns - 1) as f32 * 4.0) / columns as f32).clamp(21.0, 29.0);
            for row in (0..PREPRESS_FLOW.len()).step_by(columns) {
                ui.horizontal(|ui| {
                    for i in row..(row + columns).min(PREPRESS_FLOW.len()) {
                        previous = Some(flow_node(ui, settings, i, egui::vec2(width, 26.0), false, previous));
                    }
                    previous = None;
                });
            }
            let index = flow_index(settings.lab.step);
            ui.strong(format!("Paso {} de 8 · {}", index + 1, PREPRESS_FLOW[index].1));
        }
    });
}

/// Same staged trapping review in a dock, alongside the document canvas.
pub(crate) fn show_embedded(app: &mut VectorcraftApp, ui: &mut Ui) {
    ui.heading("Trapping");
    if app.session.active().is_none() {
        ui.label("Abre un PDF para detectar encuentros.");
        return;
    }
    let mut settings = app.ui.trapping.clone();
    refresh(app, ui, &mut settings, true);
    ui.add_space(6.0);
    embedded_steps(ui, &mut settings);
    widgets::divider(ui);
    let available = ui.available_height();
    let pin = available > 280.0;
    let footer_room = if pair(&settings).is_some() && decision_stage(settings.lab.step) { 148.0 } else { 84.0 };
    let height = if pin { (available - footer_room).min(480.0) } else { available.max(150.0) };
    egui::ScrollArea::vertical()
        .id_salt(("prepress-stage-content", settings.lab.step))
        .max_height(if pin { height } else { available })
        .auto_shrink([false, false])
        .show(ui, |ui| {
            stage_content(app, ui, &mut settings, height, true);
            if settings.failed || (settings.lab.step == 5 && settings.export_failed) {
                ui.strong("No se pudo completar");
            }
            if !settings.message.is_empty() {
                ui.label(&settings.message);
            }
            if settings.lab.step == 5 && !settings.export_message.is_empty() {
                ui.label(&settings.export_message);
            }
            if !pin {
                widgets::divider(ui);
                footer(app, ui, &mut settings, true);
            }
        });
    if pin {
        widgets::divider(ui);
        footer(app, ui, &mut settings, true);
    }
    app.ui.trapping = settings;
}

fn show_contents(app: &mut VectorcraftApp, ui: &mut Ui, settings: &mut Settings) {
    refresh(app, ui, settings, false);
    for style in [egui::TextStyle::Body, egui::TextStyle::Button] {
        if let Some(font) = ui.style_mut().text_styles.get_mut(&style) {
            font.size = font.size.max(14.0);
        }
    }
    ui.spacing_mut().item_spacing = egui::vec2(10.0, 7.0);
    let tokens = Tokens::get(ui.ctx());
    let title =
        app.session.active().map(|st| st.doc.title.chars().take(96).collect::<String>()).unwrap_or_else(|| "Abre un documento para comenzar".into());
    ui.strong(title);
    ui.label("Un paso a la vez. El dibujo cambia únicamente al incorporar la franja.");
    widgets::divider(ui);
    let compact = ui.available_width() < 760.0;
    if compact {
        egui::ComboBox::from_label("Paso")
            .selected_text(format!("{}. {}", settings.lab.step + 1, STEPS.get(settings.lab.step).unwrap_or(&STEPS[0])))
            .width(ui.available_width().min(360.0))
            .show_ui(ui, |ui| {
                for (i, name) in STEPS.iter().enumerate() {
                    if ui
                        .add_enabled(allowed(settings, i), egui::Button::new(format!("{}. {name}", i + 1)).selected(settings.lab.step == i))
                        .clicked()
                    {
                        settings.lab.step = i;
                    }
                }
            });
    }
    let decision_row = pair(settings).is_some() && (1..=4).contains(&settings.lab.step);
    let body_height = (ui.available_height() - if decision_row { 170.0 } else { 132.0 }).max(100.0);
    ui.horizontal_top(|ui| {
        if !compact {
            ui.allocate_ui_with_layout(egui::vec2(175.0, body_height), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(175.0);
                egui::ScrollArea::vertical().id_salt("lab-stage-navigation").max_height(body_height).min_scrolled_height(body_height).show(
                    ui,
                    |ui| {
                        for (i, name) in STEPS.iter().enumerate() {
                            let text = format!("{}. {name}", i + 1);
                            if ui
                                .add_enabled(
                                    allowed(settings, i),
                                    egui::Button::new(text).selected(settings.lab.step == i).min_size(egui::vec2(172.0, 38.0)),
                                )
                                .clicked()
                            {
                                settings.lab.step = i;
                            }
                            let status = if completed(settings, i) {
                                "Listo"
                            } else if allowed(settings, i) {
                                "Disponible"
                            } else {
                                "Completa el paso anterior"
                            };
                            ui.label(egui::RichText::new(status).small().color(tokens.text_dim));
                            ui.add_space(6.0);
                        }
                        ui.add_space(12.0);
                        ui.label("Desarrollo local");
                        ui.label("La prueba de tintas compartidas queda pendiente.");
                    },
                );
            });
            let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, body_height), egui::Sense::hover());
            ui.painter().line_segment([rect.left_top(), rect.left_bottom()], egui::Stroke::new(1.0, tokens.divider));
        }
        let width = ui.available_width().max(1.0);
        ui.allocate_ui_with_layout(egui::vec2(width, body_height), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_width(width);
            egui::ScrollArea::vertical().id_salt("lab-stage-content").max_height(body_height).min_scrolled_height(body_height).show(ui, |ui| {
                ui.set_min_height(body_height);
                let step = settings.lab.step;
                ui.heading(format!("{}. {}", step + 1, STEPS.get(step).unwrap_or(&STEPS[0])));
                if app.session.active().is_none() {
                    ui.label("Abre tu archivo desde Archivo > Abrir. Esta ventana permanecerá disponible.");
                    return;
                }
                stage_content(app, ui, settings, body_height, false);
            });
        });
    });
    widgets::divider(ui);
    egui::ScrollArea::vertical().id_salt("lab-status").max_height(66.0).min_scrolled_height(54.0).show(ui, |ui| {
        let message = if settings.export_failed { &settings.export_message } else { &settings.message };
        let failed = settings.failed || settings.export_failed;
        if failed {
            ui.strong("No se pudo completar");
        }
        let friendly = if message.starts_with("invalid parameters for `") {
            message.split_once(": ").map_or(message.as_str(), |(_, v)| v)
        } else {
            message.as_str()
        };
        if !friendly.is_empty() {
            ui.label(egui::RichText::new(friendly).color(tokens.text));
        }
        if !settings.export_message.is_empty() && !settings.export_failed {
            ui.label(&settings.export_message);
        }
    });
    footer(app, ui, settings, false);
}

fn stage_content(app: &mut VectorcraftApp, ui: &mut Ui, settings: &mut Settings, height: f32, docked: bool) {
    match settings.lab.step {
        0 => {
            ui.label("Calcula el mapa de encuentros visibles. No necesitas seleccionar todo el trabajo.");
            number(ui, "Resolución del detector", &mut settings.boundary_dpi, 36.0..=600.0, " ppp");
            number(ui, "Contraste mínimo", &mut settings.boundary_contrast, 5.0..=100.0, " %");
            invalidate_boundaries(app, settings);
            ui.add_space(14.0);
            ui.label("Comienza con 72 ppp y 15 %. Los candidatos no son traps ni confirman que haga falta trapping.");
            ui.label("Captura por tintas y Aplicar a la selección siguen disponibles en la paleta del editor completo.");
            if settings.boundaries.is_some() {
                ui.strong("El mapa está listo. Puedes continuar a Zona y par o detectarlo nuevamente.");
            }
        }
        1 => {
            ui.label("Elige una zona y después el par concreto que vas a revisar.");
            selectors(app, ui, settings);
            scope_controls(ui, settings);
            boundary_map(app, ui, settings, (height - 180.0).max(100.0), true);
        }
        2 => {
            context(ui, settings);
            if curves_current(settings)
                && let Some(c) = settings.curves.as_ref()
            {
                ui.label(format!(
                    "{} muestras visibles · {} inciertas · {} con papel",
                    c.report["visibleSamples"], c.report["unresolvedSamples"], c.report["gapSamples"]
                ));
                if c.report["visibleSamples"].as_u64().unwrap_or(0) == 0 {
                    ui.label(if c.scoped {
                        "No se verificó un contacto en el recuadro. Vuelve a Zona y par y dibuja uno que abarque ambos lados del borde."
                    } else {
                        "No se verificó una frontera visible. Elige otro par o revisa los ajustes antes de continuar."
                    });
                }
                if c.report["status"] == "incomplete" {
                    ui.label("Verificación incompleta. Revisa sus muestras y límites antes de continuar.");
                }
            } else {
                ui.label("Comprueba el contorno original del par elegido. Verde indicará muestras verificadas, no el ancho de trapping.");
            }
            boundary_map(app, ui, settings, (height - 155.0).max(100.0), false);
        }
        ADJUST => {
            context(ui, settings);
            ui.label("Define el ancho de la franja y hacia qué tinta se extenderá.");
            adjust_controls(ui, settings);
            direction(ui, settings);
        }
        EXPORT_PDF => {
            if settings.lab.receipt.is_some() {
                ui.strong("La franja incorporada irá en la copia del PDF.");
            }
            ui.label("Abre los ajustes de exportación y guarda una copia del documento actual.");
            ui.label("Revisa las separaciones de salida antes de usarla en producción.");
        }
        3 => {
            context(ui, settings);
            let compact = ui.available_width() < 760.0;
            if compact && preview_current(settings) {
                preview_image(ui, settings, (height - 90.0).clamp(100.0, 220.0), true);
            }
            if docked {
                ui.label(format!("Ancho: {:.2} mm · Edita el ancho y el sentido en el paso 4.", settings.preview_width_mm));
            } else {
                adjust_controls(ui, settings);
            }
            direction(ui, settings);
            if preview_current(settings) {
                if !compact {
                    preview_image(ui, settings, (height - 210.0).max(100.0), false);
                }
            } else {
                ui.label("Pulsa Previsualizar franja. Cambiar ancho o dirección requiere una nueva vista antes de incorporar.");
            }
        }
        4 => {
            context(ui, settings);
            let compact = ui.available_width() < 760.0;
            if compact && preview_current(settings) {
                preview_image(ui, settings, (height - 90.0).clamp(100.0, 220.0), true);
            }
            direction(ui, settings);
            if preview_current(settings) {
                ui.strong(format!("Franja revisada · {:.2} mm", settings.preview_width_mm));
                ui.label("Incorporar crea un vector con la tinta de origen en sobreimpresión, en una capa de trapping local. Se puede deshacer en un paso.");
                if !compact {
                    preview_image(ui, settings, (height - 210.0).max(100.0), false);
                }
            } else {
                ui.label("No hay una franja vigente. Vuelve a Previsualizar para revisar otra vista antes de incorporarla.");
            }
        }
        5 => {
            let omissions = omission_reports(settings);
            if !omissions.is_empty() {
                ui.strong(format!("{} pares omitidos por decisión del operador", omissions.len()));
                ui.label("Decisiones de este análisis. No se guardan en el archivo; se descartan al cambiar el arte o detectar nuevamente.");
                for o in omissions {
                    ui.label(format!("Zona {} · Par {} · {}", o["zone"], o["pair"], o["objects"].as_str().unwrap_or("")));
                }
                if ui.button("Revisar omisiones en Zona y par").clicked() {
                    settings.lab.step = 1;
                }
            }
            if let Some(receipt) = settings.lab.receipt.as_ref() {
                ui.strong("Franja local incorporada");
                ui.label(format!("Objeto {} · capa {} · {} mm", receipt.report["id"], receipt.report["layerId"], receipt.report["widthMm"]));
                ui.label("El vector está seleccionado en el documento. Edición > Deshacer revierte esta incorporación en un paso.");
            }
            ui.label("Exporta el diagnóstico para revisar ajustes, tintas, par y resultados vigentes. También puedes hacerlo después de un error.");
            ui.label("Este JSON no reemplaza el archivo original ni contiene sus imágenes o todos sus contornos.");
            ui.add_space(12.0);
            if docked && ui.button("Continuar a exportar PDF").clicked() {
                settings.lab.step = EXPORT_PDF;
            }
            if ui.button("Volver a Detectar").clicked() {
                settings.lab.step = 0;
            }
            if !settings.message.is_empty() {
                ui.collapsing("Detalles técnicos del último resultado", |ui| {
                    ui.label(&settings.message);
                });
            }
        }
        _ => {}
    }
}

fn adjust_controls(ui: &mut Ui, settings: &mut Settings) {
    let before = (settings.preview_width_mm, settings.preview_reverse);
    ui.horizontal_wrapped(|ui| {
        number(ui, "Ancho", &mut settings.preview_width_mm, 0.01..=0.5, " mm");
        ui.checkbox(&mut settings.preview_reverse, "Invertir origen y receptor");
    });
    if before != (settings.preview_width_mm, settings.preview_reverse) {
        settings.preview = None;
    }
    if settings.preview.is_none() && settings.lab.step == ADJUST && before != (settings.preview_width_mm, settings.preview_reverse) {
        settings.lab.adjusted = None;
    }
}

fn selectors(app: &mut VectorcraftApp, ui: &mut Ui, settings: &mut Settings) {
    let Some(c) = settings.boundaries.clone() else { return };
    let zones = c.report["zones"].as_array();
    if zones.is_none_or(Vec::is_empty) {
        ui.label("No se encontraron zonas con estos ajustes. Vuelve a Detectar para revisar el mapa.");
    }
    let selected = settings.boundary_selected.map_or_else(|| "Elige una zona".into(), |i| format!("Zona {}", i + 1));
    egui::ComboBox::from_label("Zona").selected_text(selected).width((ui.available_width() - 64.0).clamp(80.0, 600.0)).show_ui(ui, |ui| {
        for (i, z) in zones.into_iter().flatten().enumerate() {
            let inks = z["plates"].as_array().into_iter().flatten().filter_map(Value::as_str).collect::<Vec<_>>().join(" / ");
            if ui.selectable_label(settings.boundary_selected == Some(i), format!("Zona {} · {inks}", i + 1)).clicked() {
                select_zone(app, settings, &c, i);
            }
        }
    });
    if let Some(zone) = settings.boundary_selected.and_then(|i| zones.and_then(|v| v.get(i))) {
        let pairs = zone["attribution"]["pairs"].as_array();
        let count = pairs.map_or(0, Vec::len);
        let selected = settings.lab.pair.map_or_else(|| "Elige un par".into(), |i| format!("Par {}", i + 1));
        egui::ComboBox::from_label("Par").selected_text(selected).width((ui.available_width() - 64.0).clamp(80.0, 600.0)).show_ui(ui, |ui| {
            for (i, p) in pairs.into_iter().flatten().enumerate() {
                if ui
                    .selectable_label(
                        settings.lab.pair == Some(i),
                        format!(
                            "Par {}{} · {}",
                            i + 1,
                            if omitted(settings, settings.boundary_selected.unwrap_or(usize::MAX), i) { " · Omitido" } else { "" },
                            label_ids(settings, &p["ids"])
                        ),
                    )
                    .clicked()
                {
                    select_pair(app, settings, i);
                }
            }
        });
        if count == 0 {
            ui.label("Zona sin pareja atribuible. Elige otra zona para continuar.");
        } else if settings.lab.pair.is_none() {
            ui.label(format!("Hay {count} pares. Elige uno antes de verificar sus curvas."));
        }
        if let Some(p) = pair(settings) {
            ui.label(label_ids(settings, &p["ids"]));
        }
    }
}
fn direction(ui: &mut Ui, settings: &Settings) {
    if let Some(p) = pair(settings) {
        let ids = p["ids"].as_array();
        let source = if settings.preview_reverse { 1 } else { 0 };
        let name = |index| {
            ids.and_then(|v| v.get(index))
                .and_then(Value::as_u64)
                .and_then(|id| settings.boundaries.as_ref()?.source_labels.get(&id))
                .cloned()
                .unwrap_or_default()
        };
        ui.label(format!("Origen: {}", name(source)));
        ui.label(format!("Receptor: {}", name(1 - source)));
    }
}

fn scope_size([x0, y0, x1, y1]: [f64; 4]) -> String {
    format!("Área acotada: {:.1} × {:.1} mm", (x1 - x0) * 25.4 / 72.0, (y1 - y0) * 25.4 / 72.0)
}

fn scope_controls(ui: &mut Ui, settings: &mut Settings) {
    if pair(settings).is_none() || current_omitted(settings) {
        settings.lab.cancel_scope();
        return;
    }
    ui.horizontal_wrapped(|ui| {
        let drawing = settings.lab.drawing_scope;
        let label = if drawing { "Cancelar recuadro" } else { "Acotar zona" };
        if ui.button(label).clicked() || (drawing && ui.input(|i| i.key_pressed(egui::Key::Escape))) {
            settings.lab.cancel_scope();
            settings.lab.drawing_scope = !drawing;
        }
        if settings.lab.review_region.is_some() && ui.button("Usar zona completa").clicked() {
            settings.lab.clear_scope();
            scope_changed(settings, "Se usará la zona completa.");
        }
    });
    if settings.lab.drawing_scope {
        ui.label("Arrastra un recuadro en el mapa sobre el contacto entre las dos tintas.");
    } else if let Some(region) = settings.lab.review_region {
        ui.label(scope_size(region));
    }
}

fn scope_changed(settings: &mut Settings, message: &str) {
    settings.curves = None;
    settings.preview = None;
    settings.failed = false;
    settings.export_failed = false;
    settings.export_message.clear();
    settings.message = message.into();
}

fn map_bounds(value: &Value) -> Option<[f64; 4]> {
    let rect = [value[0].as_f64()?, value[1].as_f64()?, value[2].as_f64()?, value[3].as_f64()?];
    (rect[2] > rect[0] && rect[3] > rect[1]).then_some(rect)
}

fn map_region(map: egui::Rect, selection: egui::Rect, [x0, y0, x1, y1]: [f64; 4]) -> [f64; 4] {
    let x = |p| x0 + (x1 - x0) * f64::from((p - map.left()) / map.width());
    let y = |p| y0 + (y1 - y0) * f64::from((p - map.top()) / map.height());
    [x(selection.left()), y(selection.top()), x(selection.right()), y(selection.bottom())]
}

fn boundary_map(app: &mut VectorcraftApp, ui: &mut Ui, settings: &mut Settings, height: f32, picking: bool) {
    let Some(c) = settings.boundaries.clone() else { return };
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut settings.boundary_show, "Marcas rojas");
        ui.add(egui::Slider::new(&mut settings.boundary_zoom, 1.0..=8.0).text("Ampliar mapa"));
        if ui.button("Encajar mapa").clicked() {
            settings.boundary_zoom = 1.0;
        }
    });
    let pixels = egui::vec2(c.texture.size()[0] as f32, c.texture.size()[1] as f32);
    let scale = (ui.available_width() / pixels.x.max(1.0)).min(height / pixels.y.max(1.0)) * settings.boundary_zoom;
    egui::ScrollArea::both().id_salt("lab-boundary-map").max_height(height).show(ui, |ui| {
        let drawing = picking && settings.lab.drawing_scope;
        let sense = if drawing { egui::Sense::drag() } else { egui::Sense::click() };
        let response = ui.add(egui::Image::new((c.texture.id(), pixels * scale)).alt_text("Mapa de zonas y curvas").sense(sense));
        if picking
            && !drawing
            && response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let w = c.texture.size()[0];
            let h = c.texture.size()[1];
            let x = ((pos.x - response.rect.left()) / response.rect.width() * w as f32).floor() as i64;
            let y = ((pos.y - response.rect.top()) / response.rect.height() * h as f32).floor() as i64;
            if let Some(i) = zone_at(&c.zone_map, w, h, x, y) {
                select_zone(app, settings, &c, i);
            }
        }
        let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
        if settings.boundary_show {
            ui.painter().image(c.overlay.id(), response.rect, uv, egui::Color32::WHITE);
        }
        if let Some(bounds @ [x0, y0, x1, y1]) = map_bounds(&c.report["region"]) {
            let point = |x: f64, y: f64| {
                response.rect.min
                    + egui::vec2(((x - x0) / (x1 - x0)) as f32 * response.rect.width(), ((y - y0) / (y1 - y0)) as f32 * response.rect.height())
            };
            let t = Tokens::get(ui.ctx());
            let zone = settings.boundary_selected.and_then(|i| c.report["zones"].as_array()?.get(i)).and_then(|z| map_bounds(&z["bounds"]));
            if curves_current(settings)
                && let Some(check) = settings.curves.as_ref()
            {
                let centers: Vec<_> =
                    check.report["fragments"].as_array().into_iter().flatten().filter_map(|f| Some(point(f[2].as_f64()?, f[3].as_f64()?))).collect();
                for p in &centers {
                    ui.painter().circle_filled(*p, 2.0, t.text);
                }
                for p in centers {
                    ui.painter().circle_filled(p, 1.25, t.axis_y);
                }
            }
            let rect = |[a, b, c, d]: [f64; 4]| egui::Rect::from_min_max(point(a, b), point(c, d));
            if let Some(z) = zone {
                ui.painter().rect_stroke(rect(z), 0.0, egui::Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
            }
            let outline = |r| {
                ui.painter().rect_stroke(r, 0.0, egui::Stroke::new(3.0, egui::Color32::BLACK), egui::StrokeKind::Inside);
                ui.painter().rect_stroke(r, 0.0, egui::Stroke::new(1.5, egui::Color32::WHITE), egui::StrokeKind::Inside);
            };
            if let Some(region) = settings.lab.review_region {
                outline(rect(region));
            }
            if drawing {
                if response.hovered() || response.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                }
                if response.drag_started() {
                    settings.lab.scope_start = ui.input(|i| i.pointer.press_origin());
                }
                if let (Some(start), Some(end)) = (settings.lab.scope_start, response.interact_pointer_pos()) {
                    let visible = response.rect.intersect(ui.clip_rect());
                    let draft = egui::Rect::from_two_pos(start, end).intersect(visible);
                    if draft.is_positive() {
                        outline(draft);
                    }
                    if response.drag_stopped() {
                        settings.lab.scope_start = None;
                        let region = map_region(response.rect, draft, bounds);
                        let [a, b, c, d] = region;
                        if draft.width() >= 3.0 && draft.height() >= 3.0 && zone.is_some_and(|[x, y, r, s]| a < r && c > x && b < s && d > y) {
                            settings.lab.review_region = Some(region);
                            settings.lab.cancel_scope();
                            scope_changed(settings, "Recuadro listo. Verifica las curvas de este tramo.");
                        } else {
                            settings.failed = true;
                            settings.message = "Dibuja un recuadro que cruce la zona elegida y abarque ambos lados del contacto.".into();
                        }
                    }
                }
            }
        }
    });
    ui.label("Rojo: candidato. Verde: muestras verificadas. El grosor de las marcas no es ancho de trapping.");
}
fn preview_image(ui: &mut Ui, settings: &mut Settings, height: f32, compact: bool) {
    let Some(preview) = settings.preview.clone() else { return };
    if compact {
        ui.label("Desplázate abajo para ver los controles de la franja.");
    } else {
        preview_controls(ui, settings);
    }
    let pixels = egui::vec2(preview.texture.size()[0] as f32, preview.texture.size()[1] as f32);
    let scale = (ui.available_width() / pixels.x.max(1.0)).min(height / pixels.y.max(1.0)) * settings.preview_zoom;
    egui::ScrollArea::both().id_salt("lab-preview-map").max_height(height).show(ui, |ui| {
        let response = ui.add(egui::Image::new((preview.texture.id(), pixels * scale)).alt_text("Franja local previsualizada"));
        if settings.preview_show {
            ui.painter().image(
                preview.overlay.id(),
                response.rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Tokens::get(ui.ctx()).accent,
            );
        }
    });
    if compact {
        preview_controls(ui, settings);
    }
    ui.label("Resaltado: geometría temporal, no color final de sobreimpresión. El contacto sigue comprobado por muestreo.");
}

fn preview_controls(ui: &mut Ui, settings: &mut Settings) {
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut settings.preview_show, "Mostrar franja resaltada");
        ui.add(egui::Slider::new(&mut settings.preview_zoom, 1.0..=16.0).text("Ampliar franja"));
        if ui.button("Encajar franja").clicked() {
            settings.preview_zoom = 1.0;
        }
    });
}

fn decision_stage(step: usize) -> bool {
    matches!(step, 1..=4 | ADJUST)
}

fn footer(app: &mut VectorcraftApp, ui: &mut Ui, settings: &mut Settings, docked: bool) {
    let stage = settings.lab.step;
    let checked = curves_current(settings);
    let visible = settings.curves.as_ref().is_some_and(|c| c.report["visibleSamples"].as_u64().unwrap_or(0) > 0);
    let is_omitted = decision_stage(stage) && current_omitted(settings);
    if pair(settings).is_some() && decision_stage(stage) {
        ui.horizontal_wrapped(|ui| {
            let label = if is_omitted { "Volver a evaluar" } else { "Omitir trapping" };
            if ui.add(egui::Button::new(label).min_size(egui::vec2(140.0, 30.0))).clicked() {
                decision_action(app, settings, "trapping.omitPair", json!({"omit":!is_omitted}));
            }
            ui.label(if is_omitted { "Omitido · sin franja" } else { "Decisión manual para este par y esta zona." });
        });
    }
    let label = match stage {
        _ if is_omitted => "Revisar otro par",
        0 => "Detectar fronteras",
        1 => "Continuar a verificar curvas",
        2 if checked && visible && docked => "Continuar a ajustar",
        2 if checked && visible => "Continuar a previsualizar",
        ADJUST => "Continuar a previsualizar",
        EXPORT_PDF => "Abrir exportación PDF…",
        2 => "Verificar curvas",
        3 if preview_current(settings) => "Continuar a incorporar",
        3 => "Previsualizar franja",
        4 if preview_current(settings) => "Incorporar esta franja",
        4 => "Volver a previsualizar",
        _ if docked => "Descargar diagnóstico (.json)",
        _ => "Exportar diagnóstico (.json)",
    };
    let enabled = app.session.active().is_some() && (stage != 1 || (pair(settings).is_some() && !settings.lab.drawing_scope));
    let mut action = false;
    if docked || ui.available_width() < 360.0 {
        let width = ui.available_width();
        let button = if docked && enabled {
            egui::Button::new(egui::RichText::new(label).strong().color(egui::Color32::WHITE)).fill(Tokens::get(ui.ctx()).accent_strong)
        } else {
            egui::Button::new(label)
        };
        action = ui.add_enabled(enabled, button.wrap().min_size(egui::vec2(width, 32.0))).clicked();
        let previous = if docked {
            PREPRESS_FLOW.iter().take(flow_index(stage)).rev().map(|&(id, _)| id).find(|&id| embedded_step_allowed(settings, id))
        } else {
            (0..stage).rev().find(|&i| allowed(settings, i))
        };
        if ui.add_enabled(previous.is_some(), egui::Button::new("Anterior")).clicked()
            && let Some(previous) = previous
        {
            settings.lab.step = previous;
        }
    } else {
        ui.horizontal(|ui| {
            if ui.add_enabled(stage > 0, egui::Button::new("Anterior").min_size(egui::vec2(94.0, 38.0))).clicked() {
                settings.lab.step = stage.saturating_sub(1);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                action = ui.add_enabled(enabled, egui::Button::new(label).min_size(egui::vec2(240.0, 38.0))).clicked();
            });
        });
    }
    if !action {
        return;
    }
    if is_omitted {
        decision_action(app, settings, "trapping.nextDevelopmentPair", json!({}));
        return;
    }
    match stage {
        0 => {
            detect_boundaries(app, ui, settings);
            settings.lab.pair = None;
            settings.lab.receipt = None;
            if settings.boundaries.is_some() {
                settings.lab.step = 1;
            }
        }
        1 => settings.lab.step = 2,
        2 if checked && visible => settings.lab.step = if docked { ADJUST } else { 3 },
        ADJUST => {
            settings.lab.adjusted = settings.curves.as_ref().map(Arc::downgrade);
            settings.lab.step = 3;
        }
        EXPORT_PDF => {
            if let Err(e) = app.run("prepress.exportPdf", json!({})) {
                settings.failed = true;
                settings.message = e;
            }
        }
        2 => {
            if let (Some(c), Some(z), Some(p)) = (settings.boundaries.clone(), settings.boundary_selected, settings.lab.pair) {
                verify_pair(app, settings, &c, z, p, true);
            }
        }
        3 if preview_current(settings) => settings.lab.step = 4,
        3 => {
            if let Some(check) = settings.curves.clone() {
                calculate_preview(app, ui, settings, &check);
            }
        }
        4 => {
            if let Some(preview) = settings.preview.clone() {
                incorporate_preview(app, ui, settings, &preview);
            } else {
                settings.lab.step = 3;
            }
        }
        _ => {
            app.ui.trapping = settings.clone();
            match app.run("trapping.exportDiagnostic", json!({})) {
                Ok(_) => {
                    settings.export_message = "Diagnóstico JSON exportado.".into();
                    settings.export_failed = false;
                }
                Err(e) if e == "cancelled" => {
                    settings.export_message.clear();
                    settings.export_failed = false;
                }
                Err(e) => {
                    settings.export_message = e;
                    settings.export_failed = true;
                }
            }
        }
    }
}

fn omission_reports(settings: &Settings) -> Vec<Value> {
    let Some(c) = settings.boundaries.as_ref() else { return Vec::new() };
    settings
        .lab
        .omissions
        .iter()
        .filter(|o| o.boundary.ptr_eq(&Arc::downgrade(c)))
        .filter_map(|o| {
            let zone = c.report["zones"].as_array()?.get(o.zone)?;
            let p = zone["attribution"]["pairs"].as_array()?.get(o.pair)?;
            Some(json!({"status":"omitted","reason":"operatorDecision","zone":o.zone+1,"pair":o.pair+1,
            "ids":p["ids"],"region":zone["bounds"],"objects":label_ids(settings,&p["ids"])}))
        })
        .collect()
}
pub(super) fn diagnostic_state(app: &VectorcraftApp, settings: &Settings, current: bool) -> Value {
    json!({"open":settings.development_window,"step":settings.lab.step+1,
        "displayStep":if app.ui.prepress.open {flow_index(settings.lab.step)+1}else{settings.lab.step+1},
        "selectedZone":current.then_some(settings.boundary_selected.map(|i|i+1)).flatten(),
        "selectedPair":current.then_some(settings.lab.pair.map(|i|i+1)).flatten(),
        "ids":if current {pair(settings).map(|p|p["ids"].clone())}else{None},
        "reviewRegion":if current {settings.lab.review_region}else{None},
        "omissions":if current {omission_reports(settings)} else {Vec::new()},
        "omissionScope":"currentCaptureOnly",
        "incorporated":if receipt_current(app,settings){settings.lab.receipt.as_ref().map(|r|&r.report)}else{None}})
}
