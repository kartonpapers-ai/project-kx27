//! Persistent Trap Color palette. No modal and no dependency on an installed WASM plug-in.
//! Geometry is calculated only on Analyze or Apply; ordinary frames read a snapshot.

use egui::Ui;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use vectorcraft_doc::Document;

use crate::theme::Tokens;
use crate::{VectorcraftApp, widgets};

mod lab;
pub(crate) use lab::show_embedded as show_prepress;
pub use lab::{next_pair as next_development_pair, set_omission as omit_development_pair, show_window as show_development_window};

pub const ID: &str = "trapping";
const DIRECTIONS: [&str; 3] = ["Claro hacia oscuro", "Invertida", "Centrada"];
const DIRECTION_IDS: [&str; 3] = ["auto", "reverse", "center"];
const BLACK: [&str; 2] = ["Sobreimprimir negro", "Aplicar trapping"];

#[derive(Debug)]
struct Analysis {
    document: Arc<Document>,
    colors: vectorcraft_color::cms::ColorSettings,
    params: Value,
    report: Value,
}

struct Capture {
    document: Arc<Document>,
    colors: vectorcraft_color::cms::ColorSettings,
    plate: String,
    dpi: f64,
    artboard: usize,
    report: Value,
    texture: egui::TextureHandle,
}

impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Capture").field("plate", &self.plate).field("dpi", &self.dpi).field("artboard", &self.artboard).finish_non_exhaustive()
    }
}

struct BoundaryCapture {
    document: Arc<Document>,
    colors: vectorcraft_color::cms::ColorSettings,
    dpi: f64,
    contrast: f64,
    artboard: usize,
    report: Value,
    texture: egui::TextureHandle,
    overlay: egui::TextureHandle,
    zone_map: Vec<u8>,
    source_labels: std::collections::BTreeMap<u64, String>,
}

impl std::fmt::Debug for BoundaryCapture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundaryCapture").field("dpi", &self.dpi).field("contrast", &self.contrast).finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct CurveCheck {
    boundary: Arc<BoundaryCapture>,
    zone: usize,
    pair: usize,
    report: Value,
}

struct LocalPreview {
    check: Arc<CurveCheck>,
    width_mm: f64,
    reverse: bool,
    report: Value,
    texture: egui::TextureHandle,
    overlay: egui::TextureHandle,
}
impl std::fmt::Debug for LocalPreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalPreview").field("width_mm", &self.width_mm).field("reverse", &self.reverse).finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub width_mm: f64,
    pub direction: usize,
    pub black: usize,
    pub difference: f64,
    #[serde(skip)]
    pub message: String,
    #[serde(skip)]
    pub failed: bool,
    #[serde(skip)]
    export_message: String,
    #[serde(skip)]
    export_failed: bool,
    #[serde(skip)]
    document: Option<u64>,
    #[serde(skip)]
    analysis: Option<Arc<Analysis>>,
    #[serde(skip)]
    analysis_page: usize,
    pub capture_dpi: f64,
    pub capture_plate: usize,
    pub capture_zoom: f32,
    #[serde(skip)]
    capture: Option<Arc<Capture>>,
    pub boundary_dpi: f64,
    pub boundary_contrast: f64,
    pub boundary_zoom: f32,
    pub boundary_show: bool,
    #[serde(skip)]
    boundaries: Option<Arc<BoundaryCapture>>,
    #[serde(skip)]
    boundary_page: usize,
    #[serde(skip)]
    boundary_selected: Option<usize>,
    #[serde(skip)]
    curves: Option<Arc<CurveCheck>>,
    pub preview_width_mm: f64,
    pub preview_reverse: bool,
    pub preview_zoom: f32,
    pub preview_show: bool,
    #[serde(skip)]
    preview: Option<Arc<LocalPreview>>,
    pub development_window: bool,
    #[serde(skip)]
    lab: lab::State,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            width_mm: 0.1,
            direction: 0,
            black: 0,
            difference: 10.0,
            message: String::new(),
            failed: false,
            export_message: String::new(),
            export_failed: false,
            document: None,
            analysis: None,
            analysis_page: 0,
            capture_dpi: 150.0,
            capture_plate: 0,
            capture_zoom: 1.0,
            capture: None,
            boundary_dpi: 72.0,
            boundary_contrast: 15.0,
            boundary_zoom: 1.0,
            boundary_show: true,
            boundaries: None,
            boundary_page: 0,
            boundary_selected: None,
            curves: None,
            preview_width_mm: 0.1,
            preview_reverse: false,
            preview_zoom: 1.0,
            preview_show: true,
            preview: None,
            development_window: false,
            lab: lab::State::default(),
        }
    }
}

impl Settings {
    fn sanitize(&mut self) {
        if !self.preview_width_mm.is_finite() {
            self.preview_width_mm = 0.1;
        }
        if !self.preview_zoom.is_finite() {
            self.preview_zoom = 1.0;
        }
        self.preview_width_mm = self.preview_width_mm.clamp(0.01, 0.5);
        self.preview_zoom = self.preview_zoom.clamp(1.0, 16.0);
        if !self.width_mm.is_finite() {
            self.width_mm = 0.1;
        }
        if !self.difference.is_finite() {
            self.difference = 10.0;
        }
        self.width_mm = self.width_mm.clamp(0.01, 2.0);
        self.difference = self.difference.clamp(0.0, 100.0);
        self.direction = self.direction.min(2);
        self.black = self.black.min(1);
        if !self.capture_dpi.is_finite() {
            self.capture_dpi = 150.0;
        }
        self.capture_dpi = self.capture_dpi.clamp(36.0, 600.0);
        if !self.capture_zoom.is_finite() {
            self.capture_zoom = 1.0;
        }
        self.capture_zoom = self.capture_zoom.clamp(1.0, 8.0);
        if !self.boundary_dpi.is_finite() {
            self.boundary_dpi = 72.0;
        }
        if !self.boundary_contrast.is_finite() {
            self.boundary_contrast = 15.0;
        }
        if !self.boundary_zoom.is_finite() {
            self.boundary_zoom = 1.0;
        }
        self.boundary_dpi = self.boundary_dpi.clamp(36.0, 600.0);
        self.boundary_contrast = self.boundary_contrast.clamp(5.0, 100.0);
        self.boundary_zoom = self.boundary_zoom.clamp(1.0, 8.0);
    }

    fn params(&self) -> Value {
        json!({
            "widthMm": self.width_mm,
            "direction": DIRECTION_IDS.get(self.direction).unwrap_or(&"auto"),
            "black": if self.black == 1 { "trap" } else { "overprint" },
            "difference": self.difference,
        })
    }
}

#[derive(Clone)]
struct PairEdit {
    width: f64,
    mode: usize,
}

fn number(ui: &mut Ui, label: &str, value: &mut f64, range: std::ops::RangeInclusive<f64>, suffix: &str) {
    ui.horizontal(|ui| {
        let label = ui.label(label);
        ui.add(egui::DragValue::new(value).speed(0.01).range(range).suffix(suffix).max_decimals(2)).labelled_by(label.id);
    });
}

fn apply(app: &mut VectorcraftApp, settings: &mut Settings, params: Value) {
    settings.analysis = None;
    match app.run("trapping.apply", params) {
        Ok(v) => {
            let count = v.get("pairs").and_then(Value::as_array).map_or(0, Vec::len);
            settings.message = format!("{count} pares actualizados. Puedes deshacer la operación.");
            settings.failed = false;
        }
        Err(e) => {
            settings.message = e;
            settings.failed = true;
        }
    }
}

pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let tokens = Tokens::get(ui.ctx());
    let mut settings = app.ui.trapping.clone();
    settings.sanitize();
    let doc_uid = app.session.active().map(|st| st.uid);
    sync_document(app, ui, &mut settings);

    if ui.button("Abrir ventana de desarrollo").clicked() {
        app.ui.trapping = settings.clone();
        if let Err(e) = app.run("trapping.developmentWindow", json!({"open":true})) {
            settings.failed = true;
            settings.message = e;
        }
        settings.development_window = app.ui.trapping.development_window;
    }
    if settings.development_window {
        ui.label("El flujo guiado está abierto en Trapping · Desarrollo.");
        app.ui.trapping = settings;
        return;
    }
    let snapshot = vectorcraft_engine::cmd::trapping::snapshot(&app.session).ok();
    let artwork = snapshot.as_ref().and_then(|v| v["artwork"].as_u64()).unwrap_or(0);
    let pairs = snapshot.as_ref().and_then(|v| v["pairs"].as_array()).cloned().unwrap_or_default();
    let local_traps = snapshot.as_ref().and_then(|v| v["localTraps"].as_array()).cloned().unwrap_or_default();
    let ids = snapshot.as_ref().map(|v| v["ids"].clone()).unwrap_or_else(|| json!([]));
    let can_apply = doc_uid.is_some() && artwork >= 2;

    let export_requested = ui
        .add_enabled(doc_uid.is_some(), egui::Button::new("Exportar diagnóstico (.json)"))
        .on_hover_text("Guarda ajustes, tintas, una muestra de objetos e informes vigentes. No incrusta imágenes ni reemplaza el archivo original.")
        .clicked();
    if !settings.export_message.is_empty() {
        let color = if settings.export_failed { tokens.error } else { tokens.text };
        ui.label(egui::RichText::new(&settings.export_message).color(color));
    }

    capture_section(app, ui, &mut settings);
    boundary_section(app, ui, &mut settings);

    widgets::section_header(ui, "Reglas para pares nuevos");
    ui.add_enabled_ui(doc_uid.is_some(), |ui| {
        number(ui, "Ancho", &mut settings.width_mm, 0.01..=2.0, " mm");
        ui.horizontal(|ui| {
            ui.label("Dirección");
            if let Some(i) = widgets::dropdown(
                ui,
                "trap-direction",
                DIRECTIONS.get(settings.direction).unwrap_or(&DIRECTIONS[0]),
                &DIRECTIONS,
                ui.available_width().clamp(90.0, 165.0),
            ) {
                settings.direction = i;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Negro K");
            if let Some(i) =
                widgets::dropdown(ui, "trap-black", BLACK.get(settings.black).unwrap_or(&BLACK[0]), &BLACK, ui.available_width().clamp(90.0, 165.0))
            {
                settings.black = i;
            }
        });
        number(ui, "Diferencia mínima", &mut settings.difference, 0.0..=100.0, " %");
    });
    if snapshot.as_ref().is_some_and(|v| v["gradients"].as_u64().unwrap_or(0) > 0) {
        ui.label("Sólido + degradado lineal: dirección fija del sólido hacia el degradado, en sobreimpresión.");
    }
    if snapshot.as_ref().is_some_and(|v| v["images"].as_u64().unwrap_or(0) > 0) {
        ui.label("Sólido + imagen opaca: sólido hacia el marco de la imagen, sin analizar los colores de sus píxeles.");
    }
    let stale = settings.analysis.as_ref().is_some_and(|a| {
        a.params != settings.params()
            || a.colors != vectorcraft_color::cms::active_settings()
            || app.session.active().is_none_or(|st| !Arc::ptr_eq(&a.document, &st.doc))
    });
    if stale {
        settings.analysis = None;
        settings.message = "El documento o las reglas cambiaron. Analiza de nuevo.".into();
        settings.failed = false;
    }
    ui.add_space(8.0);
    if ui.add_enabled(can_apply, egui::Button::new("Analizar contactos").min_size(egui::vec2(ui.available_width(), 28.0))).clicked() {
        let params = settings.params();
        let mut p = params.clone();
        p["ids"] = ids.clone();
        settings.analysis = None;
        match app.run("trapping.analyze", p) {
            Ok(report) => {
                if let Some(st) = app.session.active() {
                    settings.analysis =
                        Some(Arc::new(Analysis { document: st.doc.clone(), colors: vectorcraft_color::cms::active_settings(), params, report }));
                    settings.analysis_page = 0;
                    settings.message = "Análisis listo. El documento no se modificó.".into();
                    settings.failed = false;
                }
            }
            Err(e) => {
                settings.message = e;
                settings.failed = true;
            }
        }
    }
    let reviewed_scope = settings.analysis.as_ref().is_none_or(|a| a.report["ids"] == ids);
    let has_contacts = settings.analysis.as_ref().is_none_or(|a| a.report["encounters"].as_array().is_some_and(|v| !v.is_empty()));
    if !reviewed_scope {
        ui.label("Recupera la selección analizada para aplicar sus reglas.");
        if ui.button("Recuperar selección analizada").clicked()
            && let Some(a) = &settings.analysis
            && let Err(e) = app.run("select.set", json!({"ids": a.report["ids"]}))
        {
            settings.message = e;
            settings.failed = true;
        }
    }
    ui.add_enabled_ui(can_apply && reviewed_scope && has_contacts, |ui| {
        if ui
            .add_sized(
                [ui.available_width(), 28.0],
                egui::Button::new(egui::RichText::new("Aplicar a la selección").strong().color(tokens.text_strong))
                    .fill(tokens.input)
                    .stroke(egui::Stroke::new(1.0, tokens.accent)),
            )
            .clicked()
        {
            let mut p = settings.params();
            p["ids"] = ids.clone();
            apply(app, &mut settings, p);
        }
        if !pairs.is_empty()
            && ui.button("Restablecer pares").on_hover_text("Recalcula los pares seleccionados y reemplaza sus ajustes individuales.").clicked()
        {
            let mut p = settings.params();
            p["ids"] = ids.clone();
            p["resetExisting"] = json!(true);
            apply(app, &mut settings, p);
        }
    });
    if !can_apply {
        ui.label(if doc_uid.is_none() {
            "Abre un documento para comenzar."
        } else {
            "Selecciona dos trazados compatibles. Para recalcular, incluye también sus traps."
        });
    } else {
        widgets::dim_label(ui, &format!("{artwork} objetos en la selección"));
    }
    if !settings.message.is_empty() {
        let color = if settings.failed { tokens.error } else { tokens.text };
        ui.label(egui::RichText::new(&settings.message).color(color));
    }
    widgets::divider(ui);
    if let Some(a) = settings.analysis.clone() {
        widgets::section_header(ui, "Análisis de contactos");
        let encounters = a.report["encounters"].as_array().map(Vec::as_slice).unwrap_or_default();
        ui.label(if encounters.len() == 1 { "1 encuentro previsto".into() } else { format!("{} encuentros previstos", encounters.len()) });
        if encounters.is_empty() {
            ui.label("No se prevén encuentros con este ancho y selección.");
        } else {
            let pages = encounters.len().div_ceil(20);
            settings.analysis_page = settings.analysis_page.min(pages.saturating_sub(1));
            if pages > 1 {
                ui.horizontal(|ui| {
                    if ui.add_enabled(settings.analysis_page > 0, egui::Button::new("Anterior")).clicked() {
                        settings.analysis_page = settings.analysis_page.saturating_sub(1);
                    }
                    ui.label(format!("{} / {pages}", settings.analysis_page + 1));
                    if ui.add_enabled(settings.analysis_page + 1 < pages, egui::Button::new("Siguiente")).clicked() {
                        settings.analysis_page += 1;
                    }
                });
            }
            egui::ScrollArea::vertical().id_salt("trap-analysis").max_height(230.0).show(ui, |ui| {
                for (index, encounter) in encounters.iter().enumerate().skip(settings.analysis_page * 20).take(20) {
                    ui.push_id(("encounter", index), |ui| {
                        let left = encounter["left"].as_str().unwrap_or_default();
                        let right = encounter["right"].as_str().unwrap_or_default();
                        let relation = encounter["relation"].as_str().unwrap_or_default();
                        let action = match relation {
                            "-" => "Sin trap".into(),
                            "=" => format!("Centrado: {left} / {right}"),
                            "S" => format!("Sobreimprimir: {left} sobre {right}"),
                            _ => format!("{left} hacia {right}"),
                        };
                        ui.strong(action);
                        ui.label(format!(
                            "{} / {}",
                            encounter["bottomName"].as_str().unwrap_or("Trazado"),
                            encounter["topName"].as_str().unwrap_or("Trazado")
                        ));
                        let reason = match encounter["reason"].as_str() {
                            Some("solidIntoImage") => "Regla fija: sólido hacia imagen opaca",
                            Some("solidIntoGradient") => "Regla fija: sólido hacia degradado",
                            Some("savedRule") => "Ajuste del par existente",
                            Some("sharedInk") => "Diferencia de tinta inferior al mínimo",
                            Some("blackOverprint") => "Regla de sobreimpresión de negro K",
                            Some("center") => "Dirección centrada",
                            Some("reverse") => "Dirección invertida",
                            _ => "Color claro hacia oscuro",
                        };
                        ui.label(format!("{:.2} mm · {reason}", encounter["widthMm"].as_f64().unwrap_or_default()));
                        if ui.button("Seleccionar pareja").clicked()
                            && let Err(e) = app.run("select.set", json!({"ids": [encounter["bottomId"], encounter["topId"]]}))
                        {
                            settings.message = e;
                            settings.failed = true;
                        }
                        widgets::divider(ui);
                    });
                }
            });
        }
        widgets::dim_label(ui, "Previsión según el ancho, las reglas y el arte seleccionado. Cada ajuste afecta a los encuentros del par mostrado.");
        widgets::divider(ui);
    }
    widgets::section_header(ui, "Traps de la selección");
    if pairs.is_empty() && local_traps.is_empty() {
        ui.label("Aplica trapping para ver y ajustar las parejas de tintas.");
    }
    for trap in &local_traps {
        ui.strong("Franja local incorporada");
        ui.label(trap["name"].as_str().unwrap_or("Trap local"));
        if trap["overprint"] == true {
            ui.label("Tinta de origen en sobreimpresión. Guardada en la capa de trapping local.");
        } else {
            ui.label("Sobreimpresión desactivada: revisa el relleno de esta franja local.");
        }
        ui.label("Puedes deshacerla con Edición > Deshacer. Para sustituirla, elimina la anterior y vuelve a previsualizar.");
    }
    for pair in &pairs {
        let Some(id) = pair["id"].as_u64() else { continue };
        let name = pair["name"].as_str().unwrap_or("Trap");
        let key = ui.id().with(("trap-pair", doc_uid, id, name, pair["widthMm"].as_f64().unwrap_or_default().to_bits(), pair["relation"].as_str()));
        let gradient = pair["solidIntoGradient"].as_bool().unwrap_or(false) || pair["solidIntoImage"].as_bool().unwrap_or(false);
        let initial = PairEdit {
            width: pair["widthMm"].as_f64().filter(|v| *v >= 0.01).unwrap_or(settings.width_mm),
            mode: if gradient {
                usize::from(pair["relation"] == "-")
            } else {
                match pair["relation"].as_str() {
                    Some("=") => 2,
                    Some("-") => 3,
                    Some("S") => 4,
                    _ => 0,
                }
            },
        };
        let mut edit = ui.ctx().data(|d| d.get_temp::<PairEdit>(key)).unwrap_or(initial);
        let left = pair["left"].as_str().unwrap_or("");
        let right = pair["right"].as_str().unwrap_or("");
        let has_black = pair["blackLeft"].as_bool().unwrap_or(false) || pair["blackRight"].as_bool().unwrap_or(false) || pair["relation"] == "S";
        let forward = format!("{left} hacia {right}");
        let reverse = format!("{right} hacia {left}");
        let modes = [forward.as_str(), reverse.as_str(), "Centrado", "Sin trap", "Sobreimprimir negro K"];
        let gradient_modes = [forward.as_str(), "Sin trap"];
        let modes = if gradient {
            &gradient_modes[..]
        } else if has_black {
            &modes[..]
        } else {
            &modes[..4]
        };
        edit.mode = edit.mode.min(modes.len().saturating_sub(1));
        ui.push_id(id, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("{left} / {right}"));
                if ui
                    .button("Localizar")
                    .on_hover_text("Selecciona el trap en el lienzo. Vuelve a seleccionar el arte y sus traps para recalcular.")
                    .clicked()
                    && let Err(e) = app.run("select.set", json!({"ids": [id]}))
                {
                    settings.message = e;
                    settings.failed = true;
                }
            });
            number(ui, "Ancho del par", &mut edit.width, 0.01..=2.0, " mm");
            if let Some(i) =
                widgets::dropdown(ui, ("trap-mode", id), modes.get(edit.mode).unwrap_or(&modes[0]), modes, ui.available_width().min(245.0))
            {
                edit.mode = i;
            }
            ui.add_enabled_ui(can_apply, |ui| {
                if ui.button("Actualizar par").clicked() {
                    let mut p = settings.params();
                    p["ids"] = ids.clone();
                    let modes = if gradient { &["forward", "none"][..] } else { &["forward", "reverse", "center", "none", "overprint"][..] };
                    let mode = modes.get(edit.mode).copied().unwrap_or("forward");
                    p["pair"] = json!({"id": id, "widthMm": edit.width, "mode": mode});
                    apply(app, &mut settings, p);
                }
            });
        });
        ui.ctx().data_mut(|d| d.insert_temp(key, edit));
        widgets::divider(ui);
    }
    ui.collapsing("Alcance de esta versión", |ui| {
        ui.label("Sólidos CMYK y spot; también sólido contra degradado lineal opaco de proceso. RGB usa conversión simple a CMYK.");
        ui.label("Los traps conservan las tintas y sobreimprimen. Para ver la aproximación en pantalla, activa View → Overprint Preview. Admite sólido hacia el marco de PNG/JPEG opacos incrustados. No admite imágenes vinculadas, transparencias, recortes, radiales, imagen contra imagen/degradado ni tintas técnicas.");
        ui.label("Los traps existentes conservan sus ajustes. Selecciona todo el arte que pueda tapar un encuentro. Máximo: 80 objetos y 12.000 nodos por operación.");
    });
    if export_requested {
        // Export the controls/reports as they stand at the end of this frame.
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
    app.ui.trapping = settings;
}

/// Bounded metadata and local curve samples, never a whole document, texture or image bytes.
fn diagnostic(app: &VectorcraftApp) -> Result<Value, String> {
    let st = app.session.active().ok_or("Abre un documento para exportar su diagnóstico.")?;
    let doc = &st.doc;
    let mut settings = app.ui.trapping.clone();
    settings.sanitize();
    let artboard = app.session.active_index().and_then(|i| app.views.get(i)).map_or(0, |v| v.artboard);
    let colors = vectorcraft_color::cms::active_settings();
    let plates = vectorcraft_doc::inks::plates(doc);
    let plate = plates.get(settings.capture_plate.min(plates.len().saturating_sub(1))).map(|p| p.name.as_str()).unwrap_or("");
    let same_document = settings.document == Some(st.uid);
    let cached = |present: bool, current: bool, report: Option<&Value>| {
        if current {
            let mut report = report.cloned().unwrap_or(Value::Null);
            if let Some(obj) = report.as_object_mut() {
                obj.remove("dataBase64");
                obj.remove("overlayBase64");
                obj.remove("zoneMapBase64");
            }
            json!({"status":"current","report":report})
        } else {
            json!({"status":if present{"stale"}else{"notAvailable"},"report":null})
        }
    };
    let analysis = settings.analysis.as_ref();
    let contacts_current =
        analysis.is_some_and(|a| same_document && Arc::ptr_eq(&a.document, doc) && a.params == settings.params() && a.colors == colors);
    let capture = settings.capture.as_ref();
    let capture_current = capture.is_some_and(|c| {
        same_document
            && Arc::ptr_eq(&c.document, doc)
            && c.colors == colors
            && c.dpi == settings.capture_dpi
            && c.plate == plate
            && c.artboard == artboard
    });
    let boundaries = settings.boundaries.as_ref();
    let boundaries_current = boundaries.is_some_and(|c| {
        same_document
            && Arc::ptr_eq(&c.document, doc)
            && c.colors == colors
            && c.dpi == settings.boundary_dpi
            && c.contrast == settings.boundary_contrast
            && c.artboard == artboard
    });
    let curves = settings.curves.as_ref();
    let curves_current = curves
        .is_some_and(|c| boundaries_current && boundaries.is_some_and(|b| Arc::ptr_eq(b, &c.boundary)) && settings.boundary_selected == Some(c.zone));
    let preview = settings.preview.as_ref();
    let preview_current = preview.is_some_and(|p| {
        curves_current
            && curves.is_some_and(|c| Arc::ptr_eq(c, &p.check))
            && p.width_mm == settings.preview_width_mm
            && p.reverse == settings.preview_reverse
    });
    let snapshot = match vectorcraft_engine::cmd::trapping::snapshot(&app.session) {
        Ok(v) => json!({"status":"available","report":v}),
        Err(e) => json!({"status":"error","message":e.to_string()}),
    };
    let commit = option_env!("GITHUB_SHA").filter(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()));
    Ok(json!({
        "format":"kx27-trapping-diagnostic","schemaVersion":1,
        "application":{"name":"Project KX27","version":env!("CARGO_PKG_VERSION"),"commit":commit},
        "document":{"title":doc.title,"sessionId":st.uid,"revision":st.revision,"colorMode":doc.color_mode,
            "sourceFormat":st.format,"activeArtboard":artboard,"artboards":doc.artboards.iter().take(100).collect::<Vec<_>>(),"artboardsTruncated":doc.artboards.len()>100},
        "colorSettings":colors,"plates":plates.iter().map(|p|json!({"name":p.name,"spot":p.spot})).collect::<Vec<_>>(),
        "settings":{"development":lab::diagnostic_state(app,&settings,boundaries_current),"trapping":settings.params(),"capture":{"dpi":settings.capture_dpi,"plate":plate},
            "boundaries":{"dpi":settings.boundary_dpi,"contrast":settings.boundary_contrast,"minPixels":4},"preview":{"widthMm":settings.preview_width_mm,"reverse":settings.preview_reverse}},
        "panelStatus":{"failed":settings.failed,"message":settings.message},
        "selection":snapshot,"objects":object_samples(doc),
        "analyses":{"contacts":cached(analysis.is_some(),contacts_current,analysis.map(|a|&a.report)),
            "capture":cached(capture.is_some(),capture_current,capture.map(|c|&c.report)),
            "boundaries":cached(boundaries.is_some(),boundaries_current,boundaries.map(|c|&c.report)),
            "curves":cached(curves.is_some(),curves_current,curves.map(|c|&c.report)),
            "preview":cached(preview.is_some(),preview_current,preview.map(|p|&p.report))},
        "fullGeometryIncluded":false,"imageBytesIncluded":false,"localCurveSamplesIncluded":curves_current,"localPreviewGeometryIncluded":preview_current,
        "notes":["Diagnóstico de solo lectura. Para reproducir el trabajo completo se necesita también el archivo original.",
            "Solo se incluyen informes vigentes ya calculados; no se recalculan contactos, capturas ni fronteras. notAvailable también puede significar que un informe se invalidó y se descartó.",
            "Las curvas pueden incluir fragmentos locales muestreados en puntos; no son contornos completos ni geometría de trapping.",
            "La previsualización puede incluir geometría local temporal de una franja; no son objetos incorporados al documento ni una prueba de impresión.",
            "Las zonas de frontera son candidatos experimentales, no confirmaciones ni objetos de trapping. Se conservan las advertencias y los límites de sus informes."]
    }))
}

fn paint_summary(p: &vectorcraft_color::Paint) -> Value {
    use vectorcraft_color::Paint;
    match p {
        Paint::None => json!({"type":"none"}),
        Paint::Solid { color, swatch, tint } => json!({"type":"solid","color":color,"swatch":swatch,"tint":tint}),
        Paint::Gradient(g) => json!({"type":"gradient","kind":g.gradient.kind,"stopCount":g.gradient.stops.len(),
            "stops":g.gradient.stops.iter().take(16).map(|s|json!({"offset":s.offset,"color":s.color,"swatch":s.swatch,"tint":s.tint,"opacity":s.opacity})).collect::<Vec<_>>(),"stopsTruncated":g.gradient.stops.len()>16}),
        Paint::Pattern { pattern, .. } => json!({"type":"pattern","name":pattern}),
    }
}

fn object_samples(doc: &Document) -> Value {
    use vectorcraft_doc::{Node, NodeKind};
    struct Sample {
        nodes: usize,
        objects: usize,
        truncated: bool,
        items: Vec<Value>,
    }
    fn visit(n: &Node, depth: usize, s: &mut Sample) {
        if s.nodes >= 20_000 || depth > 128 {
            s.truncated = true;
            return;
        }
        s.nodes += 1;
        if !matches!(n.kind, NodeKind::Layer { .. } | NodeKind::Group { .. }) {
            s.objects += 1;
            if s.items.len() < 200 {
                let paints: Vec<_> = n
                    .appearance
                    .items
                    .iter()
                    .take(16)
                    .map(|it| {
                        json!({"kind":if it.is_fill(){"fill"}else{"stroke"},
                    "paint":paint_summary(it.paint()),"opacity":it.opacity(),"overprint":it.overprint()})
                    })
                    .collect();
                s.items.push(json!({"id":n.id.0,"name":n.display_name(),"kind":n.kind_label(),"visible":n.visible,"locked":n.locked,
                    "opacity":n.opacity,"hasOpacityMask":n.extras().mask.is_some(),"paints":paints,"paintsTruncated":n.appearance.items.len()>16}));
            }
        }
        for child in n.children().into_iter().flatten() {
            if s.nodes >= 20_000 {
                s.truncated = true;
                break;
            }
            visit(child, depth + 1, s);
        }
    }
    let mut s = Sample { nodes: 0, objects: 0, truncated: false, items: vec![] };
    for layer in &doc.layers {
        if s.nodes >= 20_000 {
            s.truncated = true;
            break;
        }
        visit(layer, 0, &mut s);
    }
    json!({"nodesScanned":s.nodes,"objectsSeen":s.objects,"sampleLimit":200,"scanLimit":20_000,"scanTruncated":s.truncated,
        "samplesTruncated":s.truncated||s.objects>s.items.len(),"samples":s.items})
}

pub(crate) fn export_diagnostic(app: &mut VectorcraftApp, p: &Value) -> Result<Value, String> {
    let path = match p.get("path") {
        None => None,
        Some(v) => {
            let path = v.as_str().filter(|s| !s.trim().is_empty()).ok_or("path debe ser una ruta de archivo válida.")?;
            Some(if path.to_ascii_lowercase().ends_with(".json") { path.to_string() } else { format!("{path}.json") })
        }
    };
    let data = diagnostic(app)?;
    let bytes = serde_json::to_vec_pretty(&data).map_err(|e| e.to_string())?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("El diagnóstico supera 8 MiB. Reduce la selección antes de exportar.".into());
    }
    let st = app.session.active().ok_or("No hay documento activo.")?;
    let title = st.path.as_deref().unwrap_or(&st.doc.title);
    let name = format!("{}-trapping.json", vectorcraft_engine::cmd::fileio::file_stem(title));
    let folder = st.path.as_deref().and_then(|p| std::path::Path::new(p).parent()).map(|p| p.to_string_lossy().to_string());
    let path = match path {
        Some(path) => path,
        None if crate::io::is_web(app) => name,
        None => crate::picks::save(app, &crate::FilePick { name, folder, filters: vec![("JSON", &["json"])] }).ok_or("cancelled")?,
    };
    // A save dialog can return a name ending in a different known format too.
    let path = if path.to_ascii_lowercase().ends_with(".json") { path } else { format!("{path}.json") };
    crate::io::write_to(&mut app.services, &path, &bytes)?;
    app.status("Diagnóstico JSON exportado.");
    Ok(json!({"path":path,"bytes":bytes.len(),"schemaVersion":1}))
}

fn boundary_textures(ui: &Ui, report: &Value) -> Result<(egui::TextureHandle, egui::TextureHandle, Vec<u8>), String> {
    let load = |key: &str| -> Result<image::RgbaImage, String> {
        let bytes = report[key].as_str().and_then(vectorcraft_format::base64_decode).ok_or("No se pudo leer el mapa de fronteras.")?;
        image::load_from_memory(&bytes).map(|im| im.to_rgba8()).map_err(|e| format!("No se pudo leer el mapa: {e}"))
    };
    let base = load("dataBase64")?;
    let overlay = load("overlayBase64")?;
    let zones = load("zoneMapBase64")?;
    let limit = ui.ctx().input(|i| i.max_texture_side).max(1);
    if base.dimensions() != zones.dimensions()
        || base.dimensions() != overlay.dimensions()
        || base.width() as usize > limit
        || base.height() as usize > limit
    {
        return Err(format!("El mapa supera el límite gráfico de {limit} píxeles por lado o sus capas no coinciden. Baja los ppp."));
    }
    let texture = |name, pixels: &image::RgbaImage| {
        let image = egui::ColorImage::from_rgba_unmultiplied([pixels.width() as usize, pixels.height() as usize], pixels.as_raw());
        ui.ctx().load_texture(name, image, egui::TextureOptions::NEAREST)
    };
    Ok((texture("kx27-boundary-base", &base), texture("kx27-boundary-overlay", &overlay), zones.pixels().map(|p| p.0[0]).collect()))
}

/// Exact reported-zone pixels plus a two-pixel picking tolerance, never bounding-box hits.
fn zone_at(map: &[u8], w: usize, h: usize, x: i64, y: i64) -> Option<usize> {
    let (mut best, mut distance, mut ambiguous) = (None, i64::MAX, false);
    for dy in -2i64..=2 {
        for dx in -2i64..=2 {
            let (px, py) = (x + dx, y + dy);
            if px < 0 || py < 0 || px >= w as i64 || py >= h as i64 {
                continue;
            }
            let Some(&z) = map.get(py as usize * w + px as usize).filter(|z| **z > 0) else {
                continue;
            };
            let d = dx * dx + dy * dy;
            if d < distance {
                best = Some(z as usize - 1);
                distance = d;
                ambiguous = false;
            } else if d == distance && best != Some(z as usize - 1) {
                ambiguous = true;
            }
        }
    }
    if ambiguous { None } else { best }
}

fn choose_zone(app: &mut VectorcraftApp, settings: &mut Settings, c: &BoundaryCapture, index: usize) {
    let Some(zone) = c.report["zones"].as_array().and_then(|a| a.get(index)) else {
        return;
    };
    settings.boundary_selected = Some(index);
    settings.lab.pair = None;
    settings.curves = None;
    settings.preview = None;
    let ids: std::collections::BTreeSet<_> = zone["attribution"]["pairs"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|p| p["ids"].as_array().into_iter().flatten())
        .filter_map(Value::as_u64)
        .collect();
    if ids.is_empty() {
        settings.failed = false;
        settings.message = "Zona sin pareja atribuible. Revisa el detalle del mapa.".into();
    } else {
        match app.run("select.set", json!({"ids":ids})) {
            Ok(_) => {
                settings.failed = false;
                settings.message = "Objetos originales resaltados. No se modificó el arte.".into();
            }
            Err(e) => {
                settings.failed = true;
                settings.message = e;
            }
        }
    }
}

fn boundary_section(app: &mut VectorcraftApp, ui: &mut Ui, settings: &mut Settings) {
    invalidate_boundaries(app, settings);
    egui::CollapsingHeader::new("Fronteras visibles · Fase 2").show(ui, |ui| {
        ui.label("Detecta encuentros entre tintas. Elige una zona para resaltar los sólidos originales relacionados.");
        let before = (settings.boundary_dpi, settings.boundary_contrast);
        number(ui, "Resolución del detector", &mut settings.boundary_dpi, 36.0..=600.0, " ppp");
        number(ui, "Contraste mínimo", &mut settings.boundary_contrast, 5.0..=100.0, " %");
        if before != (settings.boundary_dpi, settings.boundary_contrast) {
            settings.boundaries = None;
            settings.curves = None;
            settings.preview = None;
            settings.boundary_selected = None;
            settings.message = "Los ajustes cambiaron. Detecta las fronteras nuevamente.".into();
            settings.failed = false;
        }
        if ui.add_enabled(app.session.active().is_some(), egui::Button::new("Detectar fronteras")).clicked() {
            detect_boundaries(app, ui, settings);
        }
        widgets::dim_label(ui, "Experimental · Máximo 1 MP y 12 tintas. Comienza con 72 ppp; el cálculo puede tardar varios segundos.");
        if let Some(c) = settings.boundaries.clone() {
            let count = c.report["zoneCount"].as_u64().unwrap_or(0);
            let label = if count == 1 { "zona candidata" } else { "zonas candidatas" };
            ui.label(format!("{count} {label}"));
            ui.label(format!("{} × {} px · {:.3} mm/píxel", c.report["width"], c.report["height"], c.report["pixelMm"].as_f64().unwrap_or(0.0)));
            ui.checkbox(&mut settings.boundary_show, "Mostrar marcas rojas");
            ui.add(egui::Slider::new(&mut settings.boundary_zoom, 1.0..=8.0).text("Ampliar mapa"));
            let width = ui.available_width().max(1.0) * settings.boundary_zoom;
            let ratio = c.report["height"].as_f64().unwrap_or(1.0) / c.report["width"].as_f64().unwrap_or(1.0).max(1.0);
            egui::ScrollArea::both().id_salt("boundary-image").max_height(260.0).show(ui, |ui| {
                let response = ui
                    .add(
                        egui::Image::new(&c.texture)
                            .alt_text("Mapa de fronteras visibles")
                            .fit_to_exact_size(egui::vec2(width, width * ratio as f32))
                            .sense(egui::Sense::click()),
                    )
                    .on_hover_text("Haz clic sobre una frontera marcada para revisar sus objetos originales.");
                if response.clicked()
                    && let Some(pos) = response.interact_pointer_pos()
                {
                    let w = c.report["width"].as_u64().unwrap_or(0) as usize;
                    let h = c.report["height"].as_u64().unwrap_or(0) as usize;
                    let x = ((pos.x - response.rect.left()) / response.rect.width() * w as f32).floor() as i64;
                    let y = ((pos.y - response.rect.top()) / response.rect.height() * h as f32).floor() as i64;
                    if let Some(index) = zone_at(&c.zone_map, w, h, x, y) {
                        choose_zone(app, settings, &c, index);
                        settings.boundary_page = index / 10;
                    }
                }
                if settings.boundary_show {
                    ui.painter().image(
                        c.overlay.id(),
                        response.rect,
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                if let Some(check) = settings.curves.as_ref().filter(|v| Arc::ptr_eq(&v.boundary, &c)) {
                    let region = &c.report["region"];
                    if let (Some(x0), Some(y0), Some(x1), Some(y1)) = (region[0].as_f64(), region[1].as_f64(), region[2].as_f64(), region[3].as_f64())
                        && x1 > x0
                        && y1 > y0
                    {
                        let point = |x: f64, y: f64| {
                            response.rect.min
                                + egui::vec2(
                                    ((x - x0) / (x1 - x0)) as f32 * response.rect.width(),
                                    ((y - y0) / (y1 - y0)) as f32 * response.rect.height(),
                                )
                        };
                        let centers: Vec<_> = check.report["fragments"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|fragment| {
                                let v = fragment.as_array().filter(|v| v.len() == 6)?;
                                Some(point(v.get(2)?.as_f64()?, v.get(3)?.as_f64()?))
                            })
                            .collect();
                        // Paint every halo first: a later halo must not erase the
                        // green centre of a neighbouring subpixel sample.
                        let tokens = Tokens::get(ui.ctx());
                        for center in &centers {
                            ui.painter().circle_filled(*center, 2.0, tokens.text);
                        }
                        for center in centers {
                            ui.painter().circle_filled(center, 1.25, tokens.axis_y);
                        }
                    }
                }
                if let Some(index) = settings.boundary_selected
                    && let Some(zone) = c.report["zones"].as_array().and_then(|a| a.get(index))
                {
                    let r = &c.report["region"];
                    let b = &zone["bounds"];
                    if let (Some(x0), Some(y0), Some(x1), Some(y1), Some(bx0), Some(by0), Some(bx1), Some(by1)) =
                        (r[0].as_f64(), r[1].as_f64(), r[2].as_f64(), r[3].as_f64(), b[0].as_f64(), b[1].as_f64(), b[2].as_f64(), b[3].as_f64())
                        && x1 > x0
                        && y1 > y0
                    {
                        let point = |x, y| {
                            response.rect.min
                                + egui::vec2(
                                    ((x - x0) / (x1 - x0)) as f32 * response.rect.width(),
                                    ((y - y0) / (y1 - y0)) as f32 * response.rect.height(),
                                )
                        };
                        ui.painter().rect_stroke(
                            egui::Rect::from_min_max(point(bx0, by0), point(bx1, by1)),
                            0.0,
                            egui::Stroke::new(2.0, Tokens::get(ui.ctx()).accent),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            });
            ui.label("Rojo = candidato. Pulsa una marca o elige una zona de la lista; se resaltan sus objetos atribuibles en el documento.");
            if let Some(zones) = c.report["zones"].as_array() {
                let pages = zones.len().div_ceil(10).max(1);
                settings.boundary_page = settings.boundary_page.min(pages - 1);
                if pages > 1 {
                    ui.horizontal(|ui| {
                        if ui.add_enabled(settings.boundary_page > 0, egui::Button::new("Anterior")).clicked() {
                            settings.boundary_page -= 1;
                        }
                        ui.label(format!("{} / {pages}", settings.boundary_page + 1));
                        if ui.add_enabled(settings.boundary_page + 1 < pages, egui::Button::new("Siguiente")).clicked() {
                            settings.boundary_page += 1;
                        }
                    });
                }
                egui::ScrollArea::vertical().id_salt("boundary-zones").max_height(140.0).show(ui, |ui| {
                    for (i, z) in zones.iter().enumerate().skip(settings.boundary_page * 10).take(10) {
                        let names =
                            z["plates"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" / ")).unwrap_or_default();
                        if ui.selectable_label(settings.boundary_selected == Some(i), format!("Zona {} · {names}", z["index"])).clicked() {
                            choose_zone(app, settings, &c, i);
                        }
                    }
                });
            }
            if let Some(index) = settings.boundary_selected
                && let Some(zone) = c.report["zones"].as_array().and_then(|a| a.get(index))
            {
                ui.strong(format!("Objetos de la zona {}", index + 1));
                let attribution = &zone["attribution"];
                let pairs = attribution["pairs"].as_array();
                let count = pairs.map_or(0, Vec::len);
                if count == 0 {
                    ui.label("Sin pareja atribuible. Puede haber transparencia, sobreimpresión, imágenes o detalles menores que el muestreo.");
                } else {
                    let summary =
                        if count == 1 { "1 par de sólidos encontrado".to_string() } else { format!("{count} pares de sólidos encontrados") };
                    ui.label(format!("{summary}. Atribución por muestreo; falta comprobar el contacto con sus curvas."));
                    if attribution["status"] == "partial" {
                        ui.label("Atribución parcial: algunas muestras no pudieron relacionarse con dos sólidos.");
                    }
                    egui::ScrollArea::vertical().id_salt("boundary-source-pairs").max_height(160.0).show(ui, |ui| {
                        for (pair_index, pair) in pairs.into_iter().flatten().enumerate() {
                            let names: Vec<_> =
                                pair["ids"].as_array().into_iter().flatten().filter_map(|id| c.source_labels.get(&id.as_u64()?).cloned()).collect();
                            ui.label(names.join(" ↔ "));
                            if ui.button(format!("Resaltar par {}", pair_index + 1)).clicked() {
                                match app.run("select.set", json!({"ids":pair["ids"]})) {
                                    Ok(_) => {
                                        settings.failed = false;
                                        settings.message = "Objetos originales resaltados. No se modificó el arte.".into();
                                    }
                                    Err(e) => {
                                        settings.failed = true;
                                        settings.message = e;
                                    }
                                }
                            }
                            if ui.button(format!("Verificar curvas del par {}", pair_index + 1)).clicked() {
                                verify_pair(app, settings, &c, index, pair_index);
                            }
                        }
                    });
                }
                if let Some(check) = settings.curves.clone().filter(|v| v.zone == index && Arc::ptr_eq(&v.boundary, &c)) {
                    ui.strong(format!("Curvas del par {}", check.pair + 1));
                    let r = &check.report;
                    let found = r["visibleSamples"].as_u64().unwrap_or(0);
                    if found > 0 {
                        ui.label(format!("{found} muestras de frontera visible sobre las curvas originales."));
                    } else {
                        ui.label("No se verificó una frontera visible con estas muestras.");
                    }
                    if r["status"] == "incomplete" {
                        ui.label("Verificación incompleta: hay muestras inciertas o se alcanzó el presupuesto.");
                    }
                    ui.label(format!(
                        "{} sondeos · {} tapados · {} con papel · {} inciertos",
                        r["testedSamples"], r["occludedSamples"], r["gapSamples"], r["unresolvedSamples"]
                    ));
                    if r["fragmentsTruncated"] == true {
                        ui.label("El mapa resume los fragmentos comprobados para mantener la revisión ágil.");
                    }
                    ui.label("Verde = muestras de la curva; no es un trap de 0,10 mm. Tolerancia de sondeo: 0,0001 mm.");
                    if found > 0 {
                        local_preview_section(app, ui, settings, &check);
                    }
                }
            }
            if c.report["zonesTruncated"] == true {
                ui.label("Informe limitado a las primeras 200 zonas. El mapa incluye todas las marcas.");
            }
            if count == 0 {
                ui.label("No se encontraron candidatos con estos ajustes; no garantiza ausencia de conflictos.");
            }
            ui.label("No confirma necesidad, dirección ni ancho. Puede omitir detalles finos y marcar textura de imágenes. No es una prueba RIP.");
        }
    });
    widgets::divider(ui);
}

fn clear_capture(_ui: &Ui, settings: &mut Settings) {
    // Dropping the last handle frees the old texture, with no image-loader cache.
    settings.capture = None;
}

fn capture_section(app: &mut VectorcraftApp, ui: &mut Ui, settings: &mut Settings) {
    let artboard = app.session.active_index().and_then(|i| app.views.get(i)).map_or(0, |v| v.artboard);
    let plates = app.session.active().map(|st| vectorcraft_doc::inks::plates(&st.doc)).unwrap_or_default();
    settings.capture_plate = settings.capture_plate.min(plates.len().saturating_sub(1));
    let plate = plates.get(settings.capture_plate).map(|p| p.name.clone()).unwrap_or_default();
    let stale = settings.capture.as_ref().is_some_and(|c| {
        c.plate != plate
            || c.dpi != settings.capture_dpi
            || c.artboard != artboard
            || c.colors != vectorcraft_color::cms::active_settings()
            || app.session.active().is_none_or(|st| !Arc::ptr_eq(&c.document, &st.doc))
    });
    if stale {
        clear_capture(ui, settings);
    }
    egui::CollapsingHeader::new("Captura por tintas · Fase 1").show(ui, |ui| {
        ui.label("Mesa completa, sin marcas de selección. No modifica el documento ni aplica trapping.");
        ui.add_enabled_ui(!plates.is_empty(), |ui| {
            let labels: Vec<&str> = plates.iter().map(|p| p.name.as_str()).collect();
            if let Some(i) = widgets::dropdown_names(ui, "capture-plate", &plate, &labels, ui.available_width().min(220.0)) {
                settings.capture_plate = i;
                clear_capture(ui, settings);
            }
            let old_dpi = settings.capture_dpi;
            number(ui, "Resolución", &mut settings.capture_dpi, 36.0..=600.0, " ppp");
            if old_dpi != settings.capture_dpi {
                clear_capture(ui, settings);
            }
            if ui.button("Capturar tinta").clicked() {
                clear_capture(ui, settings);
                let plate = plates.get(settings.capture_plate).map(|p| p.name.clone()).unwrap_or_default();
                match app.run("trapping.capture", json!({"plate":plate,"dpi":settings.capture_dpi,"artboard":artboard})) {
                    Ok(mut report) => {
                        let pixels = report["dataBase64"]
                            .as_str()
                            .and_then(vectorcraft_format::base64_decode)
                            .and_then(|bytes| image::load_from_memory(&bytes).ok())
                            .map(|image| image.to_rgba8());
                        if let (Some(pixels), Some(st)) = (pixels, app.session.active()) {
                            let limit = ui.ctx().input(|i| i.max_texture_side).max(1);
                            if pixels.width() as usize > limit || pixels.height() as usize > limit {
                                settings.message = format!(
                                    "La captura supera el límite gráfico de {limit} píxeles por lado. Baja la resolución y captura de nuevo."
                                );
                                settings.failed = true;
                                return;
                            }
                            // Upload once on explicit capture. This build installs SVG loaders
                            // only, so PNGs use the same direct texture path as other panels.
                            let color =
                                egui::ColorImage::from_rgba_unmultiplied([pixels.width() as usize, pixels.height() as usize], pixels.as_raw());
                            let texture = ui.ctx().load_texture("kx27-technical-capture", color, egui::TextureOptions::NEAREST);
                            if let Some(obj) = report.as_object_mut() {
                                obj.remove("dataBase64");
                            }
                            settings.capture = Some(Arc::new(Capture {
                                document: st.doc.clone(),
                                colors: vectorcraft_color::cms::active_settings(),
                                plate: plate.clone(),
                                dpi: settings.capture_dpi,
                                artboard,
                                report,
                                texture,
                            }));
                            settings.message = "Captura experimental lista. El documento no se modificó.".into();
                            settings.failed = false;
                        } else {
                            settings.message = "No se pudo leer la captura.".into();
                            settings.failed = true;
                        }
                    }
                    Err(e) => {
                        settings.message = e;
                        settings.failed = true;
                    }
                }
            }
        });
        widgets::dim_label(
            ui,
            "Experimental · Negro = tinta; blanco = sin tinta. Sobreimpresión aproximada. No es una prueba RIP ni detecta conflictos.",
        );
        if let Some(c) = &settings.capture {
            ui.label(format!(
                "{} · {} × {} px · {:.3} mm/píxel",
                c.plate,
                c.report["width"],
                c.report["height"],
                c.report["pixelMm"].as_f64().unwrap_or(0.0)
            ));
            ui.add(egui::Slider::new(&mut settings.capture_zoom, 1.0..=8.0).text("Ampliación"));
            let width = ui.available_width().max(1.0) * settings.capture_zoom;
            let ratio = c.report["height"].as_f64().unwrap_or(1.0) / c.report["width"].as_f64().unwrap_or(1.0).max(1.0);
            egui::ScrollArea::both().id_salt("capture-image").max_height(280.0).show(ui, |ui| {
                ui.add(egui::Image::new(&c.texture).fit_to_exact_size(egui::vec2(width, width * ratio as f32)));
            });
            ui.label("Los detalles menores que un píxel pueden perderse. Ampliar esta imagen no aumenta su resolución.");
            if !plates.iter().any(|p| p.spot) {
                ui.label("Sin muestras spot: esta captura no puede recuperar tintas planas convertidas a RGB o CMYK.");
            }
        }
    });
    widgets::divider(ui);
}

#[cfg(test)]
mod source_picking_tests {
    use super::zone_at;
    #[test]
    fn picks_marked_pixels_and_rejects_empty_boxes_and_equal_distance_zones() {
        let mut map = vec![0u8; 100];
        map[22] = 1;
        map[26] = 2;
        assert_eq!(zone_at(&map, 10, 10, 2, 2), Some(0));
        assert_eq!(zone_at(&map, 10, 10, 3, 2), Some(0));
        assert_eq!(zone_at(&map, 10, 10, 4, 2), None);
        assert_eq!(zone_at(&map, 10, 10, 6, 2), Some(1));
        assert_eq!(zone_at(&map, 10, 10, 8, 8), None);
        assert_eq!(zone_at(&map, 10, 10, -4, 0), None);
    }
}

fn preview_textures(ui: &Ui, report: &Value) -> Result<(egui::TextureHandle, egui::TextureHandle), String> {
    let load = |key: &str| -> Result<image::RgbaImage, String> {
        let bytes = report[key].as_str().and_then(vectorcraft_format::base64_decode).ok_or("No se pudo leer la previsualización.")?;
        image::load_from_memory(&bytes).map(|im| im.to_rgba8()).map_err(|e| e.to_string())
    };
    let base = load("dataBase64")?;
    let overlay = load("overlayBase64")?;
    let limit = ui.ctx().input(|i| i.max_texture_side).max(1);
    if base.dimensions() != overlay.dimensions() || base.width() as usize > limit || base.height() as usize > limit {
        return Err(format!("La previsualización supera el límite gráfico de {limit} píxeles por lado."));
    }
    let texture = |name, pixels: &image::RgbaImage| {
        ui.ctx().load_texture(
            name,
            egui::ColorImage::from_rgba_unmultiplied([pixels.width() as usize, pixels.height() as usize], pixels.as_raw()),
            egui::TextureOptions::NEAREST,
        )
    };
    Ok((texture("kx27-preview-base", &base), texture("kx27-preview-band", &overlay)))
}

fn local_preview_section(app: &mut VectorcraftApp, ui: &mut Ui, settings: &mut Settings, check: &Arc<CurveCheck>) {
    widgets::divider(ui);
    ui.strong("Previsualización local de trapping");
    ui.label("Revisa la franja antes de crear objetos. Esta vista no modifica el documento.");
    let before = (settings.preview_width_mm, settings.preview_reverse);
    number(ui, "Ancho de previsualización", &mut settings.preview_width_mm, 0.01..=0.5, " mm");
    ui.checkbox(&mut settings.preview_reverse, "Invertir origen y receptor");
    let mut ids = check.report["ids"].as_array().cloned().unwrap_or_default();
    if settings.preview_reverse {
        ids.reverse();
    }
    let label = |id: Option<&Value>| id.and_then(Value::as_u64).and_then(|id| check.boundary.source_labels.get(&id)).cloned().unwrap_or_default();
    ui.label(format!("Origen: {}", label(ids.first())));
    ui.label(format!("Receptor: {}", label(ids.get(1))));
    let stale = settings
        .preview
        .as_ref()
        .is_some_and(|p| !Arc::ptr_eq(&p.check, check) || p.width_mm != settings.preview_width_mm || p.reverse != settings.preview_reverse);
    if stale || before != (settings.preview_width_mm, settings.preview_reverse) {
        settings.preview = None;
    }
    if ui.button("Previsualizar franja").clicked() {
        calculate_preview(app, ui, settings, check);
    }
    if let Some(preview) = settings.preview.clone() {
        ui.label(format!("Franja temporal · {:.2} mm", preview.width_mm));
        ui.checkbox(&mut settings.preview_show, "Mostrar franja resaltada");
        ui.add(egui::Slider::new(&mut settings.preview_zoom, 1.0..=16.0).text("Ampliar franja"));
        let width = ui.available_width().max(1.0) * settings.preview_zoom;
        let size = egui::vec2(width, width * preview.texture.size()[1] as f32 / preview.texture.size()[0].max(1) as f32);
        egui::ScrollArea::both().id_salt("local-trap-preview").max_height(280.0).show(ui, |ui| {
            let response = ui.add(egui::Image::new((preview.texture.id(), size)));
            if settings.preview_show {
                ui.painter().image(
                    preview.overlay.id(),
                    response.rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    Tokens::get(ui.ctx()).accent,
                );
            }
        });
        ui.label("Resaltado = franja geométrica; no representa el color de sobreimpresión. Desactívalo para comparar con el original.");
        ui.label(format!("{} tramos originales · {} intervalos revisados", preview.report["originalCurveRuns"], preview.report["testedIntervals"]));
        ui.label("Uniones redondas y extremos rectos. El muestreo puede omitir detalles o extremos pequeños; la zona limita el contacto revisado.");
        ui.label(
            "Incorporar crea esta franja con la tinta de origen en sobreimpresión, en una capa de trapping local. Se puede deshacer en un paso.",
        );
        if ui.button("Incorporar esta franja").clicked() {
            incorporate_preview(app, ui, settings, &preview);
        }
        ui.label("Puedes exportar el diagnóstico JSON. Aplicar a la selección conserva su cálculo anterior.");
    }
}

fn sync_document(app: &VectorcraftApp, ui: &Ui, settings: &mut Settings) {
    settings.sanitize();
    let doc_uid = app.session.active().map(|st| st.uid);
    if settings.document != doc_uid {
        clear_capture(ui, settings);
        settings.boundaries = None;
        settings.curves = None;
        settings.preview = None;
        settings.boundary_selected = None;
        settings.analysis = None;
        settings.message.clear();
        settings.failed = false;
        settings.export_message.clear();
        settings.export_failed = false;
        settings.document = doc_uid;

        settings.lab = lab::State::default();
    }
}
fn invalidate_boundaries(app: &VectorcraftApp, settings: &mut Settings) {
    let artboard = app.session.active_index().and_then(|i| app.views.get(i)).map_or(0, |v| v.artboard);
    let stale = settings.boundaries.as_ref().is_some_and(|c| {
        c.dpi != settings.boundary_dpi
            || c.contrast != settings.boundary_contrast
            || c.artboard != artboard
            || c.colors != vectorcraft_color::cms::active_settings()
            || app.session.active().is_none_or(|st| !Arc::ptr_eq(&c.document, &st.doc))
    });
    if stale {
        settings.boundaries = None;
        settings.curves = None;
        settings.preview = None;
        settings.boundary_selected = None;
        settings.message = "El documento o los ajustes cambiaron. Detecta las fronteras nuevamente.".into();
        settings.failed = false;
    }
}
fn detect_boundaries(app: &mut VectorcraftApp, ui: &Ui, settings: &mut Settings) {
    let artboard = app.session.active_index().and_then(|i| app.views.get(i)).map_or(0, |v| v.artboard);

    settings.boundaries = None;
    settings.curves = None;
    settings.preview = None;
    settings.boundary_selected = None;
    let result = app.run("trapping.boundaries", json!({"dpi":settings.boundary_dpi,"contrast":settings.boundary_contrast,"artboard":artboard}));
    let result = result.and_then(|mut report| {
        let (texture, overlay, zone_map) = boundary_textures(ui, &report)?;
        if let Some(obj) = report.as_object_mut() {
            obj.remove("dataBase64");
            obj.remove("overlayBase64");
            obj.remove("zoneMapBase64");
        }
        let source_labels = report["sourceObjects"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|o| {
                let id = o["id"].as_u64()?;
                let inks = o["plates"].as_array().into_iter().flatten().filter_map(Value::as_str).collect::<Vec<_>>().join(" + ");
                Some((id, format!("{} · {inks}", o["name"].as_str().unwrap_or("Objeto"))))
            })
            .collect();
        let st = app.session.active().ok_or("No hay documento activo.")?;
        Ok(BoundaryCapture {
            document: st.doc.clone(),
            colors: vectorcraft_color::cms::active_settings(),
            dpi: settings.boundary_dpi,
            contrast: settings.boundary_contrast,
            artboard,
            report,
            texture,
            overlay,
            zone_map,
            source_labels,
        })
    });
    match result {
        Ok(c) => {
            settings.boundaries = Some(Arc::new(c));
            settings.boundary_page = 0;
            settings.failed = false;
            settings.message = "Mapa de candidatos listo. El documento no se modificó.".into();
        }
        Err(e) => {
            settings.message = e;
            settings.failed = true;
        }
    }
}
fn verify_pair(app: &mut VectorcraftApp, settings: &mut Settings, c: &Arc<BoundaryCapture>, zone: usize, pair: usize) {
    let Some(z) = c.report["zones"].as_array().and_then(|v| v.get(zone)) else { return };
    let Some(p) = z["attribution"]["pairs"].as_array().and_then(|v| v.get(pair)) else { return };
    settings.lab.pair = Some(pair);
    settings.curves = None;
    settings.preview = None;
    match app.run("trapping.verifyCurves", json!({"ids":p["ids"],"region":z["bounds"]})) {
        Ok(report) => {
            settings.curves = Some(Arc::new(CurveCheck { boundary: c.clone(), zone, pair, report }));
            settings.failed = false;
            settings.message = "Curvas comprobadas por muestreo. El documento no se modificó.".into();
        }
        Err(e) => {
            settings.failed = true;
            settings.message = e;
        }
    }
}
fn calculate_preview(app: &mut VectorcraftApp, ui: &Ui, settings: &mut Settings, check: &Arc<CurveCheck>) {
    let mut ids = check.report["ids"].as_array().cloned().unwrap_or_default();
    if settings.preview_reverse {
        ids.reverse();
    }

    settings.preview = None;
    let result = (|| {
        let mut report = app.run(
            "trapping.previewLocal",
            json!({"ids":ids,"region":check.report["region"],"widthMm":settings.preview_width_mm,"zoneDpi":check.boundary.dpi}),
        )?;
        let (texture, overlay) = preview_textures(ui, &report)?;
        if let Some(obj) = report.as_object_mut() {
            obj.remove("dataBase64");
            obj.remove("overlayBase64");
        }
        Ok::<_, String>(LocalPreview {
            check: check.clone(),
            width_mm: settings.preview_width_mm,
            reverse: settings.preview_reverse,
            report,
            texture,
            overlay,
        })
    })();
    match result {
        Ok(preview) => {
            settings.preview = Some(Arc::new(preview));
            settings.failed = false;
            settings.message = "Franja temporal lista. No se crearon objetos de trapping.".into();
        }
        Err(e) => {
            settings.failed = true;
            settings.message = e;
        }
    }
}
fn incorporate_preview(app: &mut VectorcraftApp, ui: &Ui, settings: &mut Settings, preview: &LocalPreview) {
    match app.run("trapping.incorporateLocal", json!({"previewId":preview.report["previewId"]})) {
        Ok(report) => {
            settings.lab.receipt = app.session.active().map(|st| lab::Receipt { uid: st.uid, document: Arc::downgrade(&st.doc), report });
            settings.lab.step = 5;
            settings.preview = None;
            settings.curves = None;
            settings.boundaries = None;
            settings.boundary_selected = None;
            settings.analysis = None;
            clear_capture(ui, settings);
            settings.failed = false;
            settings.message = "Franja incorporada en KX27 · Trapping local, con sobreimpresión. Puedes deshacer en un paso.".into();
        }
        Err(e) => {
            settings.failed = true;
            settings.message = e;
            settings.preview = None;
        }
    }
}
