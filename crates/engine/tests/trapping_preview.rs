#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use serde_json::{Value, json};
use std::sync::Arc;
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Appearance, Node, NodeId};
use vectorcraft_engine::Session;
use vectorcraft_geom::{BezPath, FillRule, Point, Rect, shapes};

fn setup() -> Session {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width":200,"height":100})).unwrap();
    s
}
fn rect(s: &mut Session, r: Rect, c: Color) -> NodeId {
    s.edit("Fixture", |d, _| {
        let id = d.alloc_id();
        let layer = d.default_layer();
        d.insert(layer, usize::MAX, Node::path(id, shapes::rectangle(r), Appearance::basic(Paint::solid(c), Paint::None, 0.0)))?;
        Ok(id)
    })
    .unwrap()
}
fn pair(s: &mut Session, gap: f64) -> [NodeId; 2] {
    [
        rect(s, Rect::new(0.0, 0.0, 100.0, 100.0), Color::cmyk(1.0, 0.0, 0.0, 0.0)),
        rect(s, Rect::new(100.0 + gap, 0.0, 200.0, 100.0), Color::cmyk(0.0, 1.0, 0.0, 0.0)),
    ]
}
fn preview(s: &mut Session, ids: [NodeId; 2], region: Value, width: f64) -> Result<Value, vectorcraft_engine::EngineError> {
    let doc = s.doc().unwrap().doc.clone();
    let sel = s.doc().unwrap().selection.objects.clone();
    let undo = s.doc().unwrap().history.undo.len();
    let redo = s.doc().unwrap().history.redo.len();
    let r = s.execute("trapping.previewLocal", &json!({"ids":ids,"region":region,"widthMm":width}));
    assert!(Arc::ptr_eq(&doc, &s.doc().unwrap().doc));
    assert_eq!(sel, s.doc().unwrap().selection.objects);
    assert_eq!(undo, s.doc().unwrap().history.undo.len());
    assert_eq!(redo, s.doc().unwrap().history.redo.len());
    r
}
fn band(r: &Value) -> BezPath {
    BezPath::from_svg(r["bandPath"].as_str().unwrap()).unwrap()
}
fn inside(b: &BezPath, x: f64, y: f64) -> bool {
    vectorcraft_geom::hit::fill_contains(b, FillRule::NonZero, Point::new(x, y))
}

#[test]
fn nominal_band_width_and_both_directions_are_geometric_and_read_only() {
    let mut s = setup();
    let ids = pair(&mut s, 0.0);
    let width = 0.1 * 72.0 / 25.4;
    for ordered in [ids, [ids[1], ids[0]]] {
        let r = preview(&mut s, ordered, json!([99, 10, 101, 90]), 0.1).unwrap();
        let b = band(&r);
        let direction = if ordered == ids { 1.0 } else { -1.0 };
        assert!(inside(&b, 100.0 + direction * width * 0.5, 50.0));
        assert!(!inside(&b, 100.0 - direction * width * 0.5, 50.0));
        assert!(!inside(&b, 100.0 + direction * width * 1.1, 50.0));
        assert!((r["bandBounds"][2].as_f64().unwrap() - r["bandBounds"][0].as_f64().unwrap() - width).abs() < 0.001);
        assert_eq!(r["status"], "previewOnly");
        assert_eq!(r["plannedOverprint"], true);
        for key in ["dataBase64", "overlayBase64"] {
            let bytes = vectorcraft_format::base64_decode(r[key].as_str().unwrap()).unwrap();
            let im = image::load_from_memory(&bytes).unwrap();
            assert!(im.width() <= 1000 && im.height() <= 1000);
        }
    }
}
#[test]
fn original_cubics_produce_curved_bands_without_joining_diagnostic_points() {
    let mut s = setup();
    let a = rect(&mut s, Rect::new(0.0, 0.0, 200.0, 100.0), Color::cmyk(1.0, 0.0, 0.0, 0.0));
    let b = s
        .edit("Ellipse", |d, _| {
            let id = d.alloc_id();
            let layer = d.default_layer();
            d.insert(
                layer,
                usize::MAX,
                Node::path(
                    id,
                    shapes::ellipse(Rect::new(60.0, 20.0, 140.0, 80.0)),
                    Appearance::basic(Paint::solid(Color::cmyk(0.0, 1.0, 0.0, 0.0)), Paint::None, 0.0),
                ),
            )?;
            Ok(id)
        })
        .unwrap();
    for (ids, sign) in [([a, b], -1.0), ([b, a], 1.0)] {
        let r = preview(&mut s, ids, json!([59, 19, 141, 81]), 0.1).unwrap();
        let curve = band(&r);
        assert!(inside(&curve, 140.0 + sign * 0.14, 50.0));
        assert!(!inside(&curve, 140.0 - sign * 0.14, 50.0));
        assert!(!inside(&curve, 100.0, 50.0));
        assert!(curve.elements().iter().any(|el| matches!(el, vectorcraft_geom::PathEl::CurveTo(..))));
        assert!(r["bandAnchors"].as_u64().unwrap() < 200, "must merge intervals before stroking: {r}");
    }
}
#[test]
fn receiver_occlusion_clips_the_entire_width_even_beyond_contact_probes() {
    for opacity in [1.0, 0.5] {
        let mut s = setup();
        let ids = pair(&mut s, 0.0);
        let c = rect(&mut s, Rect::new(100.1, 40.0, 110.0, 60.0), Color::cmyk(0.0, 0.0, 1.0, 0.0));
        s.edit("Occluder", |d, _| {
            d.node_mut(c).unwrap().opacity = opacity;
            Ok(())
        })
        .unwrap();
        let r = preview(&mut s, ids, json!([99, 10, 101, 90]), 0.1).unwrap();
        let b = band(&r);
        assert!(inside(&b, 100.05, 50.0));
        assert!(!inside(&b, 100.2, 50.0));
        assert!(inside(&b, 100.2, 20.0));
    }
}
#[test]
fn holes_and_clip_boundaries_remain_receiver_constraints() {
    for svg in [
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><defs><clipPath id="c"><rect x="100" width="100" height="100"/></clipPath></defs><rect width="200" height="100" fill="#00ffff"/><g clip-path="url(#c)"><rect width="200" height="100" fill="#ff00ff"/></g></svg>"##,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><rect width="200" height="100" fill="#ff00ff"/><path fill="#00ffff" fill-rule="evenodd" d="M0 0H200V100H0Z M100 10H150V90H100Z"/></svg>"##,
    ] {
        let mut s = setup();
        s.execute("document.open", &json!({"name":"synthetic.svg","dataBase64":vectorcraft_format::base64_encode(svg.as_bytes())})).unwrap();
        let map = s.execute("trapping.boundaries", &json!({})).unwrap();
        let ids = &map["zones"][0]["attribution"]["pairs"][0]["ids"];
        let ids = [NodeId(ids[0].as_u64().unwrap()), NodeId(ids[1].as_u64().unwrap())];
        let r = preview(&mut s, ids, json!([99, 20, 101, 80]), 0.1).unwrap();
        let b = band(&r);
        assert!(inside(&b, 100.14, 50.0) || inside(&b, 99.86, 50.0));
        assert!(!(inside(&b, 100.14, 50.0) && inside(&b, 99.86, 50.0)));
    }
}
#[test]
fn linked_spot_identity_and_large_documents_are_preserved() {
    let mut s = setup();
    for _ in 0..90 {
        rect(&mut s, Rect::new(-100.0, -100.0, -90.0, -90.0), Color::WHITE);
    }
    let ids = pair(&mut s, 0.0);
    for (id, name) in [(ids[0], "Synthetic Spot A"), (ids[1], "Synthetic Spot B")] {
        s.execute("swatch.new", &json!({"name":name,"spot":true,"color":{"model":"cmyk","c":1,"m":0,"y":0,"k":0}})).unwrap();
        s.execute("paint.setFill", &json!({"ids":[id],"swatch":name,"tint":75})).unwrap();
    }
    let r = preview(&mut s, ids, json!([99, 10, 101, 90]), 0.1).unwrap();
    assert!(r["sourcePaint"].to_string().contains("Synthetic Spot A"));
    assert!(r["sourcePaint"].to_string().contains("0.75"));
    assert_eq!(r["localObjects"], 2);
}
#[test]
fn gaps_unsupported_requests_and_budgets_fail_without_partial_geometry() {
    let mut s = setup();
    let ids = pair(&mut s, 0.01);
    assert!(preview(&mut s, ids, json!([99, 10, 101, 90]), 0.1).is_err());
    for width in [0.0, -1.0, 0.501, 1.0] {
        assert!(preview(&mut s, ids, json!([99, 10, 101, 90]), width).is_err());
    }
    assert!(preview(&mut s, [ids[0], ids[0]], json!([99, 10, 101, 90]), 0.1).is_err());
    assert!(preview(&mut s, ids, json!([101, 10, 99, 90]), 0.1).is_err());
    let mut s = setup();
    let ids = pair(&mut s, 0.0);
    for i in 0..32 {
        rect(&mut s, Rect::new(95.0, 20.0 + i as f64, 99.5, 20.0 + i as f64 + 0.2), Color::WHITE);
    }
    assert!(preview(&mut s, ids, json!([99, 10, 101, 90]), 0.1).is_err());
}

fn ellipse(s: &mut Session, bounds: Rect, c: Color) -> NodeId {
    s.edit("Synthetic ellipse", |d, _| {
        let id = d.alloc_id();
        let layer = d.default_layer();
        d.insert(layer, usize::MAX, Node::path(id, shapes::ellipse(bounds), Appearance::basic(Paint::solid(c), Paint::None, 0.0)))?;
        Ok(id)
    })
    .unwrap()
}
fn curve_ends(path: &BezPath) -> Vec<Point> {
    use vectorcraft_geom::PathEl;
    path.elements()
        .iter()
        .filter_map(|el| match el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) | PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => Some(*p),
            _ => None,
        })
        .collect()
}
#[test]
fn raster_candidate_halo_and_curve_crossings_reach_both_ellipse_junctions() {
    use vectorcraft_geom::{ParamCurve, kurbo};
    let mut s = setup();
    s.execute("file.new", &json!({"width":260,"height":200})).unwrap();
    let a = ellipse(&mut s, Rect::new(40.0, 40.0, 160.0, 160.0), Color::cmyk(1.0, 0.0, 0.0, 0.0));
    let b = ellipse(&mut s, Rect::new(100.0, 40.0, 220.0, 160.0), Color::cmyk(0.0, 0.0, 1.0, 0.0));
    // Equal translated ellipses meet on x=130. Solve the original cubic against
    // that line, independently of the preview's recursive curve/curve cutter.
    let original = shapes::ellipse(Rect::new(40.0, 40.0, 160.0, 160.0)).to_bezpath();
    let probe = kurbo::Line::new((130.0, 0.0), (130.0, 200.0));
    let junctions: Vec<_> =
        original.segments().flat_map(|seg| seg.intersect_line(probe).into_iter().map(move |hit| seg.eval(hit.segment_t))).collect();
    assert_eq!(junctions.len(), 2);
    let before = s.doc().unwrap().doc.clone();
    for dpi in [36.0, 72.0, 150.0] {
        let map = s.execute("trapping.boundaries", &json!({"dpi":dpi})).unwrap();
        let z = map["zones"]
            .as_array()
            .unwrap()
            .iter()
            .find(|z| z["attribution"]["pairs"].as_array().is_some_and(|p| p.iter().any(|p| p["ids"] == json!([a, b]))))
            .unwrap();
        for width in [0.01, 0.1, 0.5] {
            let r = s.execute("trapping.previewLocal", &json!({"ids":[a,b],"region":z["bounds"],"widthMm":width,"zoneDpi":dpi})).unwrap();
            let curve = band(&r);
            let ends = curve_ends(&curve);
            for p in &junctions {
                let distance = ends.iter().map(|q| q.distance(*p)).fold(f64::INFINITY, f64::min);
                assert!(distance < 0.003, "junction {p:?} missed by {distance} pt at dpi {dpi}, width {width}; bounds {}", r["bandBounds"]);
            }
            assert!(inside(&curve, 100.0 + width * 72.0 / 25.4 * 0.5, 100.0));
            assert!(!inside(&curve, 100.0 - width * 72.0 / 25.4 * 0.5, 100.0));
            assert!(!inside(&curve, 100.0 + width * 72.0 / 25.4 * 1.2, 100.0));
            assert!(r["bandAnchors"].as_u64().unwrap() < 100);
            assert_eq!(r["endpointMethod"], "boundedCurveIntersections");
        }
    }
    assert!(Arc::ptr_eq(&before, &s.doc().unwrap().doc));
}
#[test]
fn curve_junctions_remain_clipped_to_paper_occluders_and_explicit_region_scope() {
    let mut s = setup();
    let a = ellipse(&mut s, Rect::new(40.0, 40.0, 160.0, 160.0), Color::cmyk(1.0, 0.0, 0.0, 0.0));
    let b = ellipse(&mut s, Rect::new(100.0, 40.0, 220.0, 160.0), Color::cmyk(0.0, 0.0, 1.0, 0.0));
    rect(&mut s, Rect::new(95.0, 90.0, 108.0, 110.0), Color::cmyk(0.0, 1.0, 0.0, 0.0));
    let r = s.execute("trapping.previewLocal", &json!({"ids":[a,b],"region":[99,48,131,152],"widthMm":0.1,"zoneDpi":72})).unwrap();
    let curve = band(&r);
    assert!(!inside(&curve, 100.1, 100.0), "must not spread over a third object");
    assert!(!inside(&curve, 98.0, 100.0), "must not leave the receiver");
    assert!(!inside(&curve, 129.9, 47.0), "must not spread over paper at the upper junction");
    // An explicit crop does not silently become a whole-object trap.
    let r = preview(&mut s, [a, b], json!([99, 115, 102, 125]), 0.1).unwrap();
    assert!(r["bandBounds"][3].as_f64().unwrap() < 126.0);
    for bad in [json!(0), json!(601), json!("72")] {
        assert!(s.execute("trapping.previewLocal", &json!({"ids":[a,b],"region":[99,48,131,152],"zoneDpi":bad})).is_err());
    }
}
