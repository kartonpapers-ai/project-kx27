//! Large, modeless development workflow over the same palette caches and commands.
//! Stages guide the operator; none of them calculate geometry during ordinary frames.
use super::*;
use std::sync::Weak;

const STEPS: [&str; 6] = ["Detectar", "Zona y par", "Verificar curvas", "Previsualizar", "Incorporar", "Diagnóstico"];
#[derive(Clone, Debug, Default)]
pub(super) struct State {
    pub step: usize,
    pub pair: Option<usize>,
    pub receipt: Option<Receipt>,
    pub omissions: Vec<Omission>,
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
        0 | 5 => true,
        1 => settings.boundaries.is_some(),
        2 => pair(settings).is_some() && !current_omitted(settings),
        3 | 4 => {
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
        _ => false,
    }
}
fn select_zone(app: &mut VectorcraftApp, settings: &mut Settings, c: &Arc<BoundaryCapture>, index: usize) {
    choose_zone(app, settings, c, index);
    let pairs = c.report["zones"].as_array().and_then(|v| v.get(index)).and_then(|z| z["attribution"]["pairs"].as_array());
    settings.lab.pair = pairs.filter(|p| p.len() == 1).map(|_| 0);
}
fn select_pair(app: &mut VectorcraftApp, settings: &mut Settings, index: usize) {
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

fn show_contents(app: &mut VectorcraftApp, ui: &mut Ui, settings: &mut Settings) {
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
    settings.lab.step = settings.lab.step.min(5);
    if !allowed(settings, settings.lab.step) {
        settings.lab.step = if settings.boundaries.is_none() {
            0
        } else if pair(settings).is_none() {
            1
        } else {
            2
        };
    }
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
                stage_content(app, ui, settings, body_height);
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
    footer(app, ui, settings);
}

fn stage_content(app: &mut VectorcraftApp, ui: &mut Ui, settings: &mut Settings, height: f32) {
    match settings.lab.step {
        0 => {
            ui.label("Calcula el mapa de encuentros visibles. No necesitas seleccionar todo el trabajo.");
            number(ui, "Resolución del detector", &mut settings.boundary_dpi, 36.0..=600.0, " ppp");
            number(ui, "Contraste mínimo", &mut settings.boundary_contrast, 5.0..=100.0, " %");
            invalidate_boundaries(app, settings);
            ui.add_space(14.0);
            ui.label("Comienza con 72 ppp y 15 %. Los candidatos no son traps ni confirman que haga falta trapping.");
            ui.label("Captura por tintas y Aplicar a la selección siguen disponibles en el panel habitual, al cerrar esta ventana.");
            if settings.boundaries.is_some() {
                ui.strong("El mapa está listo. Puedes continuar a Zona y par o detectarlo nuevamente.");
            }
        }
        1 => {
            ui.label("Elige una zona y después el par concreto que vas a revisar.");
            selectors(app, ui, settings);
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
                    ui.label("No se verificó una frontera visible. Elige otro par o revisa los ajustes antes de continuar.");
                }
                if c.report["status"] == "incomplete" {
                    ui.label("Verificación incompleta. Revisa sus muestras y límites antes de continuar.");
                }
            } else {
                ui.label("Comprueba el contorno original del par elegido. Verde indicará muestras verificadas, no el ancho de trapping.");
            }
            boundary_map(app, ui, settings, (height - 155.0).max(100.0), false);
        }
        3 => {
            context(ui, settings);
            let compact = ui.available_width() < 760.0;
            if compact && preview_current(settings) {
                preview_image(ui, settings, (height - 90.0).clamp(100.0, 220.0), true);
            }
            let before = (settings.preview_width_mm, settings.preview_reverse);
            ui.horizontal_wrapped(|ui| {
                number(ui, "Ancho", &mut settings.preview_width_mm, 0.01..=0.5, " mm");
                ui.checkbox(&mut settings.preview_reverse, "Invertir origen y receptor");
            });
            if before != (settings.preview_width_mm, settings.preview_reverse) {
                settings.preview = None;
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
        let response = ui.add(egui::Image::new((c.texture.id(), pixels * scale)).alt_text("Mapa de zonas y curvas").sense(egui::Sense::click()));
        if picking
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
        let r = &c.report["region"];
        if let (Some(x0), Some(y0), Some(x1), Some(y1)) = (r[0].as_f64(), r[1].as_f64(), r[2].as_f64(), r[3].as_f64())
            && x1 > x0
            && y1 > y0
        {
            let point = |x: f64, y: f64| {
                response.rect.min
                    + egui::vec2(((x - x0) / (x1 - x0)) as f32 * response.rect.width(), ((y - y0) / (y1 - y0)) as f32 * response.rect.height())
            };
            let t = Tokens::get(ui.ctx());
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
            if let Some(z) = settings.boundary_selected.and_then(|i| c.report["zones"].as_array()?.get(i)) {
                let b = &z["bounds"];
                if let (Some(a), Some(b), Some(c), Some(d)) = (b[0].as_f64(), b[1].as_f64(), b[2].as_f64(), b[3].as_f64()) {
                    ui.painter().rect_stroke(
                        egui::Rect::from_min_max(point(a, b), point(c, d)),
                        0.0,
                        egui::Stroke::new(2.0, t.accent),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
    });
    ui.label("Rojo: candidato. Verde: muestras verificadas. El grosor de las marcas no es ancho de trapping.");
}
fn preview_image(ui: &mut Ui, settings: &mut Settings, height: f32, compact: bool) {
    let Some(preview) = settings.preview.clone() else { return };
    if compact {
        ui.label("Desplázate abajo para ajustar ancho, sentido y ampliación.");
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

fn footer(app: &mut VectorcraftApp, ui: &mut Ui, settings: &mut Settings) {
    let stage = settings.lab.step;
    let checked = curves_current(settings);
    let visible = settings.curves.as_ref().is_some_and(|c| c.report["visibleSamples"].as_u64().unwrap_or(0) > 0);
    let is_omitted = (1..=4).contains(&stage) && current_omitted(settings);
    if pair(settings).is_some() && (1..=4).contains(&stage) {
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
        2 if checked && visible => "Continuar a previsualizar",
        2 => "Verificar curvas",
        3 if preview_current(settings) => "Continuar a incorporar",
        3 => "Previsualizar franja",
        4 if preview_current(settings) => "Incorporar esta franja",
        4 => "Volver a previsualizar",
        _ => "Exportar diagnóstico (.json)",
    };
    let enabled = app.session.active().is_some() && (stage != 1 || pair(settings).is_some());
    let mut action = false;
    ui.horizontal(|ui| {
        if ui.add_enabled(stage > 0, egui::Button::new("Anterior").min_size(egui::vec2(94.0, 38.0))).clicked() {
            settings.lab.step = stage.saturating_sub(1);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            action = ui.add_enabled(enabled, egui::Button::new(label).min_size(egui::vec2(240.0, 38.0))).clicked();
        });
    });
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
        2 if checked && visible => settings.lab.step = 3,
        2 => {
            if let (Some(c), Some(z), Some(p)) = (settings.boundaries.clone(), settings.boundary_selected, settings.lab.pair) {
                verify_pair(app, settings, &c, z, p);
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
        "selectedZone":current.then_some(settings.boundary_selected.map(|i|i+1)).flatten(),
        "selectedPair":current.then_some(settings.lab.pair.map(|i|i+1)).flatten(),
        "ids":if current {pair(settings).map(|p|p["ids"].clone())}else{None},
        "omissions":if current {omission_reports(settings)} else {Vec::new()},
        "omissionScope":"currentCaptureOnly",
        "incorporated":if receipt_current(app,settings){settings.lab.receipt.as_ref().map(|r|&r.report)}else{None}})
}
