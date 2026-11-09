#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use vectorcraft_engine::Session;
use vectorcraft_ui_egui::{
    VectorcraftApp,
    panels::trapping::show_development_window,
    theme::{self, Brightness},
};

type Downloads = Rc<RefCell<Vec<Vec<u8>>>>;
fn fixture(shared: bool, multiple: bool) -> (VectorcraftApp, Downloads) {
    let mut app = VectorcraftApp::new(Session::new(), Default::default());
    app.run("file.new", json!({"width":200,"height":150})).unwrap();
    for (x, y, w, h, color) in [
        (
            0,
            0,
            100,
            100,
            if shared { json!({"model":"cmyk","c":0,"m":0.06,"y":0.87,"k":0.01}) } else { json!({"model":"cmyk","c":1,"m":0,"y":0,"k":0}) },
        ),
        (30, 30, 40, 40, json!({"model":"cmyk","c":0,"m":1,"y":0,"k":0})),
    ] {
        app.run("shape.rectangle", json!({"x":x,"y":y,"width":w,"height":h})).unwrap();
        app.run("paint.setFill", json!({"color":color})).unwrap();
        app.run("paint.setStroke", json!({"none":true})).unwrap();
    }
    if multiple {
        app.run("shape.rectangle", json!({"x":50,"y":40,"width":40,"height":40})).unwrap();
        app.run("paint.setFill", json!({"color":{"model":"cmyk","c":0,"m":0,"y":1,"k":0}})).unwrap();
        app.run("paint.setStroke", json!({"none":true})).unwrap();
    }
    app.run("select.all", json!({})).unwrap();
    app.run("trapping.developmentWindow", json!({})).unwrap();
    let files = Rc::new(RefCell::new(Vec::new()));
    let copied = files.clone();
    app.services.download = Some(Box::new(move |_, bytes| copied.borrow_mut().push(bytes.to_vec())));
    (app, files)
}
fn harness(app: VectorcraftApp, w: f32, h: f32, theme: Brightness, gpu: bool) -> Harness<'static, VectorcraftApp> {
    let builder = Harness::builder().with_size(egui::vec2(w, h));
    let builder = if gpu { builder.wgpu() } else { builder };
    let mut ready = false;
    let mut test = builder.build_ui_state(
        move |ui, app: &mut VectorcraftApp| {
            if !ready {
                theme::install_fonts(ui.ctx());
                egui_extras::install_image_loaders(ui.ctx());
                theme::apply(ui.ctx(), theme);
                ready = true;
                return;
            }
            show_development_window(app, ui.ctx());
        },
        app,
    );
    test.run_steps(5);
    test
}
fn click(h: &mut Harness<'_, VectorcraftApp>, label: &str) {
    h.get_by_role_and_label(egui::accesskit::Role::Button, label).click();
    h.run_steps(4);
}
fn choose_first(h: &mut Harness<'_, VectorcraftApp>) {
    h.get_by_label("Zona").click();
    h.run_steps(4);
    h.get_by_label_contains("Zona 1 ·").click();
    h.run_steps(4);
}
fn verify_first(h: &mut Harness<'_, VectorcraftApp>) {
    click(h, "Detectar fronteras");
    assert!(!h.state().ui.trapping.failed, "{}", h.state().ui.trapping.message);
    choose_first(h);
    click(h, "Continuar a verificar curvas");
    click(h, "Verificar curvas");
    assert!(!h.state().ui.trapping.failed, "{}", h.state().ui.trapping.message);
}
fn build_preview(h: &mut Harness<'_, VectorcraftApp>) {
    verify_first(h);
    click(h, "Continuar a previsualizar");
    click(h, "Previsualizar franja");
    assert!(!h.state().ui.trapping.failed, "{}", h.state().ui.trapping.message);
}
fn footer_visible(h: &Harness<'_, VectorcraftApp>, label: &str, w: f64, height: f64) {
    let n = h.get_by_role_and_label(egui::accesskit::Role::Button, label);
    let rect = n.accesskit_node().bounding_box().unwrap();
    assert!(rect.x0 >= 0.0 && rect.y0 >= 0.0 && rect.x1 <= w && rect.y1 <= height, "footer is outside the viewport: {rect:?}");
}

#[test]
fn development_command_is_modeless_read_only_and_visibility_roundtrips() {
    let (mut app, _) = fixture(false, false);
    let before = app.session.doc().unwrap().doc.clone();
    let undo = app.session.doc().unwrap().history.undo.len();
    assert!(app.ui.dialog.is_none());
    assert!(app.ui.trapping.development_window);
    let saved = serde_json::to_value(&app.ui).unwrap();
    let restored: vectorcraft_ui_egui::state::UiState = serde_json::from_value(saved).unwrap();
    assert!(restored.trapping.development_window);
    assert!(serde_json::to_value(&restored.trapping).unwrap().get("lab").is_none());
    app.run("trapping.developmentWindow", json!({"open":false})).unwrap();
    assert!(!app.ui.trapping.development_window);
    assert!(app.run("trapping.developmentWindow", json!({"open":"bad"})).is_err());
    assert!(Arc::ptr_eq(&before, &app.session.doc().unwrap().doc));
    assert_eq!(undo, app.session.doc().unwrap().history.undo.len());
}
#[test]
fn ordered_development_flow_preserves_art_until_explicit_incorporation_and_exports_context() {
    let (app, files) = fixture(false, false);
    let before = app.session.doc().unwrap().doc.clone();
    let undo = app.session.doc().unwrap().history.undo.len();
    let mut h = harness(app, 1024.0, 720.0, Brightness::Light, false);
    assert!(h.query_by_label("Aplicar a la selección").is_none());
    footer_visible(&h, "Detectar fronteras", 1024.0, 720.0);
    build_preview(&mut h);
    assert!(Arc::ptr_eq(&before, &h.state().session.doc().unwrap().doc));
    assert_eq!(undo, h.state().session.doc().unwrap().history.undo.len());
    assert!(h.query_by_label("Incorporar esta franja").is_none(), "incorporation has its own explicit review step");
    click(&mut h, "Continuar a incorporar");
    footer_visible(&h, "Incorporar esta franja", 1024.0, 720.0);
    click(&mut h, "Incorporar esta franja");
    assert!(!h.state().ui.trapping.failed, "{}", h.state().ui.trapping.message);
    h.get_by_label("Franja local incorporada");
    footer_visible(&h, "Exportar diagnóstico (.json)", 1024.0, 720.0);
    assert_eq!(h.state().session.doc().unwrap().history.undo.len(), undo + 1);
    click(&mut h, "Exportar diagnóstico (.json)");
    let d: Value = serde_json::from_slice(&files.borrow()[0]).unwrap();
    assert_eq!(d["settings"]["development"]["step"], 6);
    assert_eq!(d["settings"]["development"]["incorporated"]["status"], "incorporated");
    assert_eq!(d["selection"]["report"]["localTraps"].as_array().unwrap().len(), 1);
    h.state_mut().run("edit.undo", json!({})).unwrap();
    h.run_steps(4);
    assert_eq!(h.state().session.doc().unwrap().doc, before);
    assert!(h.query_by_label("Franja local incorporada").is_none());
}
#[test]
fn drawing_a_review_box_limits_the_preview_and_is_recorded_after_incorporation() {
    let (app, files) = fixture(false, false);
    let before = app.session.doc().unwrap().doc.clone();
    let mut h = harness(app, 1024.0, 800.0, Brightness::MediumDark, false);
    click(&mut h, "Detectar fronteras");
    choose_first(&mut h);
    click(&mut h, "Acotar zona");
    let map = h.get_by_label("Mapa de zonas y curvas").accesskit_node().bounding_box().unwrap();
    let pos = |x: f64, y: f64| egui::pos2((map.x0 + (map.x1 - map.x0) * x / 200.) as f32, (map.y0 + (map.y1 - map.y0) * y / 150.) as f32);
    let (start, end) = (pos(40., 26.), pos(57., 34.));
    h.hover_at(start);
    h.drag_at(start);
    h.run_steps(3);
    h.hover_at(end);
    h.run_steps(3);
    h.drop_at(end);
    h.run_steps(4);
    h.get_by_label("Usar zona completa");
    click(&mut h, "Continuar a verificar curvas");
    click(&mut h, "Verificar curvas");
    click(&mut h, "Continuar a previsualizar");
    click(&mut h, "Previsualizar franja");
    assert!(!h.state().ui.trapping.failed, "{}", h.state().ui.trapping.message);
    assert!(Arc::ptr_eq(&before, &h.state().session.doc().unwrap().doc));
    h.state_mut().run("trapping.exportDiagnostic", json!({})).unwrap();
    let report: Value = serde_json::from_slice(&files.borrow()[0]).unwrap();
    let region = &report["settings"]["development"]["reviewRegion"];
    for (actual, expected) in region.as_array().unwrap().iter().zip([40., 26., 57., 34.]) {
        assert!((actual.as_f64().unwrap() - expected).abs() < 0.75, "{region}");
    }
    assert_eq!(report["analyses"]["curves"]["report"]["region"], *region);
    assert_eq!(report["analyses"]["preview"]["report"]["region"], *region);
    click(&mut h, "Continuar a incorporar");
    click(&mut h, "Incorporar esta franja");
    h.get_by_label("Franja local incorporada");
    click(&mut h, "Exportar diagnóstico (.json)");
    let added: Value = serde_json::from_slice(&files.borrow()[1]).unwrap();
    assert_eq!(added["settings"]["development"]["incorporated"]["reviewRegion"], *region);
    assert_eq!(added["selection"]["report"]["localTraps"].as_array().unwrap().len(), 1);
}

#[test]
fn changed_direction_and_document_invalidate_only_the_required_steps() {
    let (app, files) = fixture(false, false);
    let mut h = harness(app, 1024.0, 720.0, Brightness::MediumDark, false);
    build_preview(&mut h);
    h.get_by_label("Invertir origen y receptor").click();
    h.run_steps(4);
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Previsualizar franja");
    assert!(h.query_by_label("Continuar a incorporar").is_none());
    click(&mut h, "6. Diagnóstico");
    click(&mut h, "Exportar diagnóstico (.json)");
    let d: Value = serde_json::from_slice(&files.borrow()[0]).unwrap();
    assert_eq!(d["settings"]["development"]["selectedZone"], 1);
    assert_eq!(d["settings"]["development"]["selectedPair"], 1);
    assert_eq!(d["analyses"]["preview"]["status"], "notAvailable");
    click(&mut h, "4. Previsualizar");
    click(&mut h, "Previsualizar franja");
    h.state_mut().run("shape.rectangle", json!({"x":2,"y":2,"width":2,"height":2})).unwrap();
    h.run_steps(4);
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Detectar fronteras");
    assert!(h.query_by_label("Incorporar esta franja").is_none());
    h.state_mut().run("file.new", json!({})).unwrap();
    h.run_steps(4);
    assert!(h.state().ui.trapping.development_window);
}
#[test]
fn shared_ink_failure_stays_visible_and_diagnostic_remains_available() {
    let (app, files) = fixture(true, false);
    let before = app.session.doc().unwrap().doc.clone();
    let mut h = harness(app, 1024.0, 720.0, Brightness::Light, false);
    build_preview(&mut h);
    click(&mut h, "Continuar a incorporar");
    click(&mut h, "Incorporar esta franja");
    assert!(h.state().ui.trapping.failed);
    assert!(h.state().ui.trapping.message.contains("tinta compartida"));
    assert!(Arc::ptr_eq(&before, &h.state().session.doc().unwrap().doc));
    footer_visible(&h, "Volver a previsualizar", 1024.0, 720.0);
    click(&mut h, "6. Diagnóstico");
    click(&mut h, "Exportar diagnóstico (.json)");
    let d: Value = serde_json::from_slice(&files.borrow()[0]).unwrap();
    assert_eq!(d["panelStatus"]["failed"], true);
    assert_eq!(d["settings"]["development"]["selectedPair"], 1);
}
#[test]
fn multiple_pairs_require_an_explicit_pair_choice_and_small_windows_keep_actions_visible() {
    let (app, _) = fixture(false, true);
    let mut h = harness(app, 700.0, 560.0, Brightness::MediumDark, false);
    click(&mut h, "Detectar fronteras");
    choose_first(&mut h);
    let next = h.get_by_role_and_label(egui::accesskit::Role::Button, "Continuar a verificar curvas");
    assert!(next.accesskit_node().is_disabled());
    h.get_by_label("Par").click();
    h.run_steps(4);
    h.get_by_label_contains("Par 2 ·").click();
    h.run_steps(4);
    footer_visible(&h, "Continuar a verificar curvas", 700.0, 560.0);
    click(&mut h, "Continuar a verificar curvas");
    footer_visible(&h, "Verificar curvas", 700.0, 560.0);
    let selected = h.state().session.doc().unwrap().selection.objects.len();
    assert_eq!(selected, 2);
}
#[test]
fn empty_document_session_can_open_and_close_without_creating_art() {
    let mut app = VectorcraftApp::new(Session::new(), Default::default());
    app.run("trapping.developmentWindow", json!({})).unwrap();
    let h = harness(app, 700.0, 560.0, Brightness::Light, false);
    h.get_by_label("Abre un documento para comenzar");
    let node = h.get_by_role_and_label(egui::accesskit::Role::Button, "Detectar fronteras");
    assert!(node.accesskit_node().is_disabled());
    assert!(h.state().session.active().is_none());
}

#[test]
fn development_native_visual_capture() {
    let Ok(dir) = std::env::var("KX27_LAB_SCREENSHOTS") else { return };
    std::fs::create_dir_all(&dir).unwrap();
    for (w, height, theme) in [(1280.0, 800.0, Brightness::MediumDark), (1024.0, 700.0, Brightness::Light), (700.0, 560.0, Brightness::MediumDark)] {
        let (app, _) = fixture(false, false);
        let mut h = harness(app, w, height, theme, true);
        let prefix = format!("{}-{}x{}", theme.id(), w as u32, height as u32);
        h.render().unwrap().save(std::path::Path::new(&dir).join(format!("{prefix}-1-detect.png"))).unwrap();
        click(&mut h, "Detectar fronteras");
        choose_first(&mut h);
        if w > 1100.0 {
            h.render().unwrap().save(std::path::Path::new(&dir).join(format!("{prefix}-2-zone.png"))).unwrap();
        }
        click(&mut h, "Continuar a verificar curvas");
        click(&mut h, "Verificar curvas");
        if w > 1100.0 {
            h.render().unwrap().save(std::path::Path::new(&dir).join(format!("{prefix}-3-curves.png"))).unwrap();
        }
        click(&mut h, "Continuar a previsualizar");
        click(&mut h, "Previsualizar franja");
        h.render().unwrap().save(std::path::Path::new(&dir).join(format!("{prefix}-4-preview.png"))).unwrap();
        click(&mut h, "Continuar a incorporar");
        h.render().unwrap().save(std::path::Path::new(&dir).join(format!("{prefix}-5-incorporate.png"))).unwrap();
        click(&mut h, "Incorporar esta franja");
        assert!(!h.state().ui.trapping.failed, "{}", h.state().ui.trapping.message);
        h.render().unwrap().save(std::path::Path::new(&dir).join(format!("{prefix}-6-diagnostic.png"))).unwrap();
    }
    let (app, _) = fixture(true, false);
    let mut h = harness(app, 1024.0, 700.0, Brightness::Light, true);
    build_preview(&mut h);
    click(&mut h, "Continuar a incorporar");
    click(&mut h, "Incorporar esta franja");
    assert!(h.state().ui.trapping.failed);
    h.render().unwrap().save(std::path::Path::new(&dir).join("light-1024x700-error.png")).unwrap();
}

#[test]
fn omit_is_read_only_reversible_and_exported_for_the_exact_pair() {
    let (app, files) = fixture(true, true);
    let before = app.session.doc().unwrap().doc.clone();
    let undo = app.session.doc().unwrap().history.undo.len();
    let mut h = harness(app, 700.0, 560.0, Brightness::Light, false);
    click(&mut h, "Detectar fronteras");
    choose_first(&mut h);
    assert!(h.query_by_label("Omitir trapping").is_none());
    h.get_by_label("Par").click();
    h.run_steps(4);
    h.get_by_label_contains("Par 1 ·").click();
    h.run_steps(4);
    footer_visible(&h, "Omitir trapping", 700.0, 560.0);
    click(&mut h, "Omitir trapping");
    h.get_by_label("Omitido · sin franja");
    footer_visible(&h, "Revisar otro par", 700.0, 560.0);
    assert!(h.query_by_label("Incorporar esta franja").is_none());
    assert_eq!(h.state().session.doc().unwrap().history.undo.len(), undo);
    assert!(Arc::ptr_eq(&before, &h.state().session.doc().unwrap().doc));
    // Omission cannot be bypassed by jumping straight to verification.
    h.state_mut().run("trapping.omitPair", json!({})).unwrap();
    click(&mut h, "Revisar otro par");
    assert!(h.query_by_label("Omitido · sin franja").is_none());
    h.state_mut().run("trapping.omitPair", json!({})).unwrap();
    h.run_steps(4);
    // Return to the original omitted pair and restore it.
    h.get_by_label("Par").click();
    h.run_steps(4);
    h.get_by_label_contains("Par 1 · Omitido").click();
    h.run_steps(4);
    click(&mut h, "Volver a evaluar");
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Continuar a verificar curvas");
    assert!(h.query_by_label("Omitido · sin franja").is_none());
    h.state_mut().run("trapping.omitPair", json!({})).unwrap();
    h.run_steps(4);
    // Use the command so the same assertion works with the compact Paso selector.
    h.state_mut().run("trapping.developmentWindow", json!({"open":false})).unwrap();
    h.state_mut().run("trapping.exportDiagnostic", json!({})).unwrap();
    let d: Value = serde_json::from_slice(&files.borrow()[0]).unwrap();
    let omissions = d["settings"]["development"]["omissions"].as_array().unwrap();
    assert_eq!(omissions.len(), 2);
    assert_eq!(omissions[0]["reason"], "operatorDecision");
    assert!(omissions.iter().all(|o| o["status"] == "omitted"));
    assert_eq!(d["selection"]["report"]["localTraps"].as_array().unwrap().len(), 0);
    assert!(Arc::ptr_eq(&before, &h.state().session.doc().unwrap().doc));
    assert!(h.state_mut().run("trapping.omitPair", json!({"omit":"yes"})).is_err());
    h.state_mut().run("shape.rectangle", json!({"x":2,"y":2,"width":2,"height":2})).unwrap();
    assert!(h.state_mut().run("trapping.omitPair", json!({})).is_err());
    h.state_mut().run("trapping.exportDiagnostic", json!({})).unwrap();
    let d: Value = serde_json::from_slice(files.borrow().last().unwrap()).unwrap();
    assert_eq!(d["settings"]["development"]["omissions"], json!([]));
}
#[test]
fn omitting_a_prepared_pair_clears_preview_and_all_omitted_opens_diagnostic() {
    let (app, files) = fixture(false, false);
    let before = app.session.doc().unwrap().doc.clone();
    let mut h = harness(app, 1024.0, 720.0, Brightness::MediumDark, false);
    build_preview(&mut h);
    click(&mut h, "Omitir trapping");
    assert!(h.query_by_label("Continuar a incorporar").is_none());
    click(&mut h, "Revisar otro par");
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Exportar diagnóstico (.json)");
    click(&mut h, "Exportar diagnóstico (.json)");
    let d: Value = serde_json::from_slice(&files.borrow()[0]).unwrap();
    assert_eq!(d["settings"]["development"]["step"], 6);
    assert_eq!(d["settings"]["development"]["omissions"][0]["ids"], json!([2, 3]));
    assert_eq!(d["analyses"]["preview"]["status"], "notAvailable");
    assert!(Arc::ptr_eq(&before, &h.state().session.doc().unwrap().doc));
    click(&mut h, "1. Detectar");
    click(&mut h, "Detectar fronteras");
    choose_first(&mut h);
    h.state_mut().run("trapping.exportDiagnostic", json!({})).unwrap();
    let d: Value = serde_json::from_slice(files.borrow().last().unwrap()).unwrap();
    assert_eq!(d["settings"]["development"]["omissions"], json!([]));
}

#[test]
fn omission_native_visual_capture() {
    let Ok(dir) = std::env::var("KX27_OMIT_SCREENSHOTS") else { return };
    std::fs::create_dir_all(&dir).unwrap();
    for (w, height, theme) in [(1280.0, 800.0, Brightness::MediumDark), (1024.0, 700.0, Brightness::Light), (700.0, 560.0, Brightness::MediumDark)] {
        let (app, _) = fixture(true, false);
        let mut h = harness(app, w, height, theme, true);
        click(&mut h, "Detectar fronteras");
        choose_first(&mut h);
        let prefix = format!("{}-{}x{}", theme.id(), w as u32, height as u32);
        for (label, action) in [("available", "Omitir trapping"), ("omitted", "Revisar otro par"), ("diagnostic", "")] {
            h.render().unwrap().save(std::path::Path::new(&dir).join(format!("{prefix}-{label}.png"))).unwrap();
            if !action.is_empty() {
                click(&mut h, action);
            }
        }
    }
}
