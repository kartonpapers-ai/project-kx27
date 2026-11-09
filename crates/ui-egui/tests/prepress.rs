#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use egui_kittest::Harness;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use vectorcraft_color::Paint;
use vectorcraft_engine::Session;
use vectorcraft_ui_egui::{VectorcraftApp, io};

fn fixture() -> VectorcraftApp {
    let mut maker = Session::new();
    maker.execute("file.new", &json!({"width":240,"height":160})).unwrap();
    for (name, c, m) in [("Spot A", 1, 0), ("Spot B", 0, 1)] {
        maker.execute("swatch.new", &json!({"name":name,"spot":true,"color":{"model":"cmyk","c":c,"m":m,"y":0,"k":0}})).unwrap();
    }
    for (x, y, paint) in [
        (20, 20, json!({"swatch":"Spot A","tint":65})),
        (100, 20, json!({"color":{"model":"cmyk","c":0,"m":0,"y":1,"k":0}})),
        (20, 105, json!({"swatch":"Spot B","tint":40})),
    ] {
        maker.execute("shape.rectangle", &json!({"x":x,"y":y,"width":80,"height":40})).unwrap();
        maker.execute("paint.setFill", &paint).unwrap();
        maker.execute("paint.setStroke", &json!({"none":true})).unwrap();
    }
    // Exercise the actual PDF reader, not the writer's optional native editing payload.
    let data = maker.execute("document.exportPdf", &json!({"preserveEditing":false})).unwrap();
    let pdf = vectorcraft_format::base64_decode(data["dataBase64"].as_str().unwrap()).unwrap();
    let mut app = VectorcraftApp::new(Session::new(), Default::default());
    app.run("prepress.workspace", json!({})).unwrap();
    io::open_bytes(&mut app, "Prueba de tintas.pdf", &pdf, None).unwrap();
    app
}

fn spot_nodes(app: &VectorcraftApp, name: &str) -> Vec<(vectorcraft_doc::NodeId, f32)> {
    let mut hits = Vec::new();
    app.session.doc().unwrap().doc.walk(|n| {
        if let Paint::Solid { swatch: Some(ink), tint, .. } = n.appearance.fill_paint()
            && ink == name
        {
            hits.push((n.id, tint));
        }
    });
    hits
}

#[test]
fn opens_pdf_maps_spots_and_exports_local_trap_to_named_plate() {
    let mut app = fixture();
    let before = app.session.doc().unwrap().doc.clone();
    let [(_, imported_a)] = spot_nodes(&app, "Spot A")[..] else { panic!("expected one Spot A") };
    let [(_, imported_b)] = spot_nodes(&app, "Spot B")[..] else { panic!("expected one Spot B") };
    // The current PDF writer quantizes spot tints to 8 bits. Mapping must keep
    // the exact tint the PDF reader obtained, including non-round percentages.
    assert!((imported_a - 0.65).abs() <= 1.0 / 255.0);
    assert!((imported_b - 0.4).abs() <= 1.0 / 255.0);
    for p in [
        json!({"source":"Spot A","newName":"Spot B"}),
        json!({"source":"Spot A","newName":"cyan"}),
        json!({"source":"Cyan","target":"Spot B"}),
        json!({"source":"Spot A","target":"Spot A"}),
    ] {
        assert!(app.run("prepress.mapSpot", p).is_err());
        assert!(Arc::ptr_eq(&before, &app.session.doc().unwrap().doc));
    }
    app.run("view.separationsPreview", json!({"only":"Spot A"})).unwrap();
    app.run("prepress.mapSpot", json!({"source":"Spot A","newName":"Azul marca"})).unwrap();
    assert_eq!(app.run("color.plates", json!({})).unwrap()["on"], false);
    assert!(spot_nodes(&app, "Spot A").is_empty());
    app.run("prepress.mapSpot", json!({"source":"Spot B","target":"Azul marca"})).unwrap();
    let mut tints: Vec<_> = spot_nodes(&app, "Azul marca").into_iter().map(|(_, tint)| tint).collect();
    tints.sort_by(f32::total_cmp);
    assert_eq!(tints, vec![imported_b, imported_a]);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(spot_nodes(&app, "Spot B").len(), 1);
    let [(source, tint)] = spot_nodes(&app, "Azul marca")[..] else { panic!("expected one source") };
    assert_eq!(tint, imported_a);
    let mut target = None;
    app.session.doc().unwrap().doc.walk(|n| {
        if n.appearance.fill_paint().color() == Some(vectorcraft_color::Color::cmyk(0., 0., 1., 0.)) {
            target = Some(n.id);
        }
    });
    let preview = app.run("trapping.previewLocal", json!({"ids":[source,target.unwrap()],"region":[99,21,101,58],"widthMm":0.1})).unwrap();
    let added = app.run("trapping.incorporateLocal", json!({"previewId":preview["previewId"]})).unwrap();
    assert_eq!(added["overprint"], true);
    assert_eq!(spot_nodes(&app, "Azul marca").len(), 2);
    let downloaded = Rc::new(RefCell::new(Vec::new()));
    let copy = downloaded.clone();
    app.services.download = Some(Box::new(move |_, bytes| copy.borrow_mut().extend_from_slice(bytes)));
    app.run("file.export.pdf", json!({"path":"preprensa.pdf","preserveEditing":false,"compression":{"compressText":false}})).unwrap();
    let bytes = downloaded.borrow();
    let pdf = String::from_utf8_lossy(&bytes);
    assert!(pdf.contains("/Separation") && pdf.contains("/Azul#20marca") && !pdf.contains("/Spot#20A"));
    assert!(pdf.contains("/OP true/op true/OPM 1"));
    let mut reimported = VectorcraftApp::new(Session::new(), Default::default());
    io::open_bytes(&mut reimported, "preprensa.pdf", &bytes, None).unwrap();
    assert_eq!(spot_nodes(&reimported, "Azul marca").iter().filter(|(_, tint)| (*tint - imported_a).abs() <= 1.0 / 255.0).count(), 2);
}

fn frame(app: &mut VectorcraftApp, ui: &mut egui::Ui) {
    app.logic(ui.ctx());
    app.ui(ui);
}

#[test]
fn prepress_frame_shows_workspace_and_switches_back_without_editing() {
    use egui_kittest::kittest::Queryable;
    let app = fixture();
    let before = app.session.doc().unwrap().doc.clone();
    let mut h = Harness::builder().with_size(egui::vec2(1200., 760.)).build_ui_state(|ui, app| frame(app, ui), app);
    h.run_steps(5);
    h.get_by_label("KX27 Preprensa");
    h.get_by_label("Mapeo de tintas");
    h.get_by_label("Detectar fronteras");
    h.get_by_label("Volver al editor").click();
    h.run_steps(4);
    assert!(h.query_by_label("KX27 Preprensa").is_none());
    assert!(Arc::ptr_eq(&before, &h.state().session.doc().unwrap().doc));
    assert_eq!(h.state_mut().run("prepress.workspace", json!({})).unwrap(), json!({"open":true}));
    let _: Value = h.state_mut().run("view.fitArtboard", json!({})).unwrap();
}

#[test]
fn prepress_guides_detection_and_downloads_the_diagnostic() {
    use egui_kittest::kittest::Queryable;
    let mut app = fixture();
    let before = app.session.doc().unwrap().doc.clone();
    let files = Rc::new(RefCell::new(Vec::new()));
    let downloads = files.clone();
    app.services.download = Some(Box::new(move |name, bytes| downloads.borrow_mut().push((name.to_owned(), bytes.to_vec()))));
    let mut h = Harness::builder().with_size(egui::vec2(1200., 760.)).build_ui_state(|ui, app| frame(app, ui), app);
    h.run_steps(5);
    h.get_by_label("1. Detectar · Actual");
    h.get_by_label("3. Verificar curvas · Pendiente");
    h.get_by_label("5. Incorporar · Pendiente");
    h.get_by_label("Detectar fronteras").click();
    h.run_steps(5);
    h.get_by_label("1. Detectar · Listo");
    h.get_by_label("2. Zona y par · Actual");
    h.get_by_label("6. Diagnóstico · Disponible").click();
    h.run_steps(4);
    h.get_by_label("Descargar diagnóstico (.json)").click();
    h.run_steps(4);
    let files = files.borrow();
    let [(name, bytes)] = files.as_slice() else { panic!("expected one download") };
    assert!(name.ends_with("-trapping.json"));
    let report: Value = serde_json::from_slice(bytes).unwrap();
    assert_eq!(report["format"], "kx27-trapping-diagnostic");
    assert_eq!(report["settings"]["development"]["step"], 6);
    assert!(report["analyses"]["boundaries"]["report"].is_object());
    assert_eq!(report["imageBytesIncluded"], false);
    assert!(Arc::ptr_eq(&before, &h.state().session.doc().unwrap().doc));
    h.get_by_label("Anterior").click();
    h.run_steps(4);
    h.get_by_label("2. Zona y par · Actual");
}

#[test]
#[ignore = "requires a graphics adapter; optional visual check"]
fn captures_prepress_workspace() {
    use egui_kittest::kittest::Queryable;
    let dir = std::env::var("KX27_PREPRESS_SHOTS").unwrap();
    for (w, h) in [(1280., 800.), (820., 640.)] {
        let mut test = Harness::builder().with_size(egui::vec2(w, h)).wgpu().build_ui_state(|ui, app| frame(app, ui), fixture());
        test.run_steps(5);
        if w < 920. {
            test.get_by_label("Tintas").click();
            test.run_steps(4);
        }
        test.render().unwrap().save(std::path::Path::new(&dir).join(format!("prepress-{w}.png"))).unwrap();
        let state = if w < 920.0 {
            test.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Spot A · Plana").focus();
            test.run_steps(4);
            assert!(test.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Spot A · Plana").is_focused());
            "focus"
        } else {
            test.get_by_role(egui::accesskit::Role::TextInput).focus();
            test.run_steps(4);
            test.get_by_role(egui::accesskit::Role::TextInput).type_text("Spot B");
            test.run_steps(4);
            test.get_by_label("Renombrar tinta").click();
            test.run_steps(4);
            test.get_by_label("No se pudo mapear la tinta");
            "error"
        };
        test.render().unwrap().save(std::path::Path::new(&dir).join(format!("prepress-{w}-{state}.png"))).unwrap();
        if w < 920.0 {
            test.get_by_label("Trapping").click();
            test.run_steps(4);
        }
        test.get_by_label("6. Diagnóstico · Disponible").click();
        test.run_steps(4);
        test.get_by_label("Descargar diagnóstico (.json)");
        test.render().unwrap().save(std::path::Path::new(&dir).join(format!("prepress-{w}-diagnostic.png"))).unwrap();
    }
}
