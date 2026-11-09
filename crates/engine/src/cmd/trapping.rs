//! Project KX27's native adapter for the steppa-trap geometry crate.
//! The panel and automation use the same validated, transactional commands.

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use vectorcraft_color::{BlendMode, Color, Paint, gradient::GradientKind};
use vectorcraft_doc::{Appearance, AppearanceItem, Document, FillLayer, ImageObject, Node, NodeKind};
use vectorcraft_geom::{Rect, Shape};
use vectorcraft_plugins::objects;
use vectorcraft_trapping::{Modo, Opciones, PT_POR_MM, analizar_con, atrapar_con, leer_ajuste};

use super::*;
mod local;
pub(crate) use local::PreparedLocalTrap;

const APPLY: &str = "trapping.apply";
const META: &str = "kx27-trap-v1";
// The original plug-in had interpreter budgets. Bound the native selection too.
const MAX_INPUTS: usize = 80;
const MAX_POINTS: usize = 12_000;

fn number(p: &Value, key: &str, default: f64, min: f64, max: f64) -> Result<f64> {
    match p.get(key) {
        None => Ok(default),
        Some(v) => v
            .as_f64()
            .filter(|n| n.is_finite() && (min..=max).contains(n))
            .ok_or_else(|| bad(APPLY, format!("{key}: introduce un valor entre {min} y {max}."))),
    }
}

fn choice<'a>(p: &'a Value, key: &str, default: &'a str) -> Result<&'a str> {
    match p.get(key) {
        None => Ok(default),
        Some(v) => v.as_str().ok_or_else(|| bad(APPLY, format!("{key} debe ser texto."))),
    }
}

fn options(p: &Value) -> Result<Opciones> {
    let modo = match choice(p, "direction", "auto")? {
        "auto" => Modo::Auto,
        "reverse" => Modo::Invertido,
        "center" => Modo::Centrado,
        _ => return Err(bad(APPLY, "direction debe ser auto, reverse o center.")),
    };
    let negro_sobreimprime = match choice(p, "black", "overprint")? {
        "overprint" => true,
        "trap" => false,
        _ => return Err(bad(APPLY, "black debe ser overprint o trap.")),
    };
    Ok(Opciones {
        ancho: number(p, "widthMm", 0.1, 0.01, 2.0)? * PT_POR_MM,
        modo,
        negro_sobreimprime,
        diferencia_minima: number(p, "difference", 10.0, 0.0, 100.0)? / 100.0,
    })
}

fn targets(s: &Session, p: &Value) -> Result<Vec<NodeId>> {
    let st = s.doc()?;
    let selected = ids_param(p, "ids").unwrap_or_else(|| st.selection.objects.clone());
    // Native-only extension of the vector plug-in target filter: include images in the
    // same paint order, without changing which objects third-party plug-ins receive.
    fn visit(list: &[std::sync::Arc<Node>], selected: &BTreeSet<NodeId>, inside: bool, out: &mut Vec<NodeId>) {
        for n in list {
            if !n.visible || n.locked {
                continue;
            }
            let picked = inside || selected.contains(&n.id);
            match &n.kind {
                NodeKind::Path { guide: false, clipping: false, .. } | NodeKind::Compound { .. } | NodeKind::Image(_) if picked => out.push(n.id),
                NodeKind::Group { children, .. } | NodeKind::Layer { children, .. } => visit(children, selected, picked, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    visit(&st.doc.layers, &selected.into_iter().collect(), false, &mut out);
    Ok(out)
}

/// Resolve links without changing the artwork. Display alternates never become spot plates.
fn encoded_color(doc: &Document, paint: &Paint) -> Result<Value> {
    let Paint::Solid { color, swatch, tint } = paint else {
        return Err(bad(APPLY, "Usa un relleno plano."));
    };
    if !tint.is_finite() || !(0.0..=1.0).contains(tint) || (swatch.is_none() && *tint != 1.0) {
        return Err(bad(APPLY, "Porcentaje de tinta inválido."));
    }
    let mut resolved = *color;
    let mut spot = false;
    if let Some(name) = swatch {
        if name == vectorcraft_color::swatch::REGISTRATION {
            return Err(bad(APPLY, "Registro no es una tinta de ilustración."));
        }
        let w = doc.swatch(name).ok_or_else(|| bad(APPLY, "La muestra vinculada no existe."))?;
        spot = w.spot;
        if spot && ["All", "None", "Cyan", "Magenta", "Yellow", "Black"].iter().any(|reserved| name.eq_ignore_ascii_case(reserved)) {
            return Err(bad(APPLY, "El nombre de la tinta está reservado por PDF. Usa un nombre spot específico."));
        }
        resolved = doc.global_color(name).ok_or_else(|| bad(APPLY, "La muestra debe ser global y de color plano."))?.tinted(*tint);
    }
    if !spot && matches!(resolved, Color::Lab { .. }) {
        let cms = vectorcraft_color::cms::active();
        resolved = cms.convert(&resolved, vectorcraft_color::cms::Model::Cmyk, cms.settings().intent);
    }
    let finite = match resolved {
        Color::Rgb { r, g, b } => [r, g, b].iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
        Color::Cmyk { c, m, y, k } => [c, m, y, k].iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
        Color::Gray { k } => k.is_finite() && (0.0..=1.0).contains(&k),
        Color::Lab { l, a, b } => l.is_finite() && (0.0..=100.0).contains(&l) && [a, b].iter().all(|v| v.is_finite() && (-128.0..=127.0).contains(v)),
    };
    if !finite {
        return Err(bad(APPLY, "El relleno contiene componentes de color inválidos."));
    }
    if spot && (swatch.as_ref().is_some_and(|n| n.len() > 256) || (*tint > 0.0 && vectorcraft_doc::overprint::is_white(&Paint::solid(resolved)))) {
        return Err(bad(APPLY, "Esta etapa admite tintas spot convencionales; blanco opaco y tintas técnicas necesitan reglas específicas."));
    }
    Ok(if spot {
        json!({"model":"spot", "name":swatch, "tint":tint, "alternate":resolved})
    } else {
        serde_json::to_value(resolved).map_err(|e| bad(APPLY, e.to_string()))?
    })
}

// A gradient is only a destination/opaque occluder in this phase. It is never sampled,
// converted, or copied into a trap. White process stops are opaque paper white, not alpha.
fn validate_gradient(doc: &Document, paint: &Paint) -> Result<()> {
    let Paint::Gradient(g) = paint else { return Err(bad(APPLY, "Se esperaba un degradado.")) };
    if g.gradient.kind != GradientKind::Linear || g.freeform.is_some() || g.swatch.is_some() {
        return Err(bad(APPLY, "Esta etapa admite degradados lineales de proceso sin vínculo a una muestra de degradado; no radiales ni libres."));
    }
    if !(2..=256).contains(&g.gradient.stops.len()) || !g.angle.is_finite() {
        return Err(bad(APPLY, "El degradado necesita entre 2 y 256 paradas y una dirección válida."));
    }
    if let Some(geom) = g.geom {
        let points = [geom.start, geom.end];
        if points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > 4e6 || p.y.abs() > 4e6)
            || !geom.aspect.is_finite()
            || geom.aspect <= 0.0
            || geom.focal.is_some()
            || geom.start.distance(geom.end) < 1e-9
        {
            return Err(bad(APPLY, "La colocación del degradado no es válida para trapping."));
        }
    }
    let mut previous = -1.0;
    for stop in &g.gradient.stops {
        if !stop.offset.is_finite()
            || !(0.0..=1.0).contains(&stop.offset)
            || stop.offset < previous
            || stop.opacity != 1.0
            || !stop.midpoint.is_finite()
            || !(0.01..=0.99).contains(&stop.midpoint)
        {
            return Err(bad(APPLY, "Usa paradas ordenadas y opacas; un degradado hacia transparente no está admitido."));
        }
        previous = stop.offset;
        let p = Paint::Solid { color: stop.color, swatch: stop.swatch.clone(), tint: stop.tint };
        if encoded_color(doc, &p)?["model"] == "spot" {
            return Err(bad(APPLY, "Los degradados spot necesitan una etapa específica. Usa un degradado de proceso."));
        }
    }
    Ok(())
}

fn source_label(doc: &Document, colors: &BTreeMap<String, vectorcraft_trapping::Color>, key: &str) -> Option<String> {
    if let Some(c) = colors.get(key) {
        return Some(display_label(c, key));
    }
    if let Some(id) = key.strip_prefix("IM").and_then(|s| s.parse::<u64>().ok()) {
        let n = doc.node(NodeId(id))?;
        return matches!(n.kind, NodeKind::Image(_)).then(|| format!("Imagen · {}", n.display_name()));
    }
    let id = key.strip_prefix("GL")?.parse::<u64>().ok()?;
    let n = doc.node(NodeId(id))?;
    matches!(n.appearance.fill_paint(), Paint::Gradient(_)).then(|| format!("Degradado · {}", n.display_name()))
}

// No pixel colour decisions yet: only prove the rectangular image is fully opaque.
// Bounded decoding happens on Analyze/Apply, never in a UI snapshot.
const MAX_IMAGE_PIXELS: u64 = 32_000_000;
fn validate_image(doc: &Document, im: &ImageObject, checked: &mut BTreeMap<String, (u32, u32)>, pixels: &mut u64, bytes: &mut usize) -> Result<()> {
    if im.link.is_some() {
        return Err(bad(APPLY, "Incrusta la imagen antes de aplicar trapping; no se usan vistas previas de vínculos."));
    }
    let blob = doc.images.get(&im.key).ok_or_else(|| bad(APPLY, "Faltan los datos de la imagen."))?;
    if blob.is_proxy() {
        return Err(bad(APPLY, "La imagen solo contiene una vista previa. Incrusta el original."));
    }
    let coefficients = im.xf.as_coeffs();
    if im.width == 0
        || im.height == 0
        || coefficients.iter().any(|v| !v.is_finite())
        || !im.xf.determinant().is_finite()
        || im.xf.determinant().abs() < 1e-12
    {
        return Err(bad(APPLY, "El tamaño o la transformación de la imagen no es válido."));
    }
    for (x, y) in [(0.0, 0.0), (im.width as f64, 0.0), (im.width as f64, im.height as f64), (0.0, im.height as f64)] {
        let p = im.xf * kurbo::Point::new(x, y);
        if !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > 4e6 || p.y.abs() > 4e6 {
            return Err(bad(APPLY, "La imagen queda fuera del rango de coordenadas."));
        }
    }
    if let Some(dim) = checked.get(&im.key) {
        if *dim != (im.width, im.height) {
            return Err(bad(APPLY, "Las dimensiones no coinciden con los píxeles de la imagen."));
        }
        return Ok(());
    }
    if bytes.saturating_add(blob.bytes.len()) > 64 * 1024 * 1024 {
        return Err(bad(APPLY, "La selección supera 64 MB de imágenes distintas. Trabaja por partes o usa imágenes optimizadas."));
    }
    let reader = image::ImageReader::new(std::io::Cursor::new(blob.bytes.as_slice())).with_guessed_format().map_err(|e| bad(APPLY, e.to_string()))?;
    let format = reader
        .format()
        .filter(|f| matches!(f, image::ImageFormat::Png | image::ImageFormat::Jpeg))
        .ok_or_else(|| bad(APPLY, "Esta etapa admite imágenes PNG o JPEG incrustadas."))?;
    let (w, h) = reader.into_dimensions().map_err(|e| bad(APPLY, format!("No se puede leer la imagen: {e}")))?;
    let count = u64::from(w) * u64::from(h);
    if (w, h) != (im.width, im.height) || count == 0 {
        return Err(bad(APPLY, "Las dimensiones no coinciden con los píxeles de la imagen."));
    }
    if pixels.saturating_add(count) > MAX_IMAGE_PIXELS {
        return Err(bad(APPLY, "La selección supera 32 megapíxeles de imágenes distintas. Trabaja por partes."));
    }
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(blob.bytes.as_slice()), format);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|e| bad(APPLY, format!("No se pueden comprobar los píxeles: {e}")))?;
    let transparent = match &decoded {
        image::DynamicImage::ImageLumaA8(p) => p.pixels().any(|p| p.0[1] != u8::MAX),
        image::DynamicImage::ImageRgba8(p) => p.pixels().any(|p| p.0[3] != u8::MAX),
        image::DynamicImage::ImageLumaA16(p) => p.pixels().any(|p| p.0[1] != u16::MAX),
        image::DynamicImage::ImageRgba16(p) => p.pixels().any(|p| p.0[3] != u16::MAX),
        image::DynamicImage::ImageRgba32F(p) => p.pixels().any(|p| p.0[3] != 1.0),
        _ => decoded.color().has_alpha(),
    };
    if transparent {
        return Err(bad(
            APPLY,
            "La imagen contiene transparencia. Esta etapa trabaja con el marco completo de imágenes opacas, no con siluetas recortadas.",
        ));
    }
    *pixels += count;
    *bytes += blob.bytes.len();
    checked.insert(im.key.clone(), (w, h));
    Ok(())
}

// Only calculation input gets a four-corner polygon. The actual image stays an Image node.
fn image_input(n: &Node, im: &ImageObject) -> Value {
    let anchors: Vec<Value> = [(0.0, 0.0), (im.width as f64, 0.0), (im.width as f64, im.height as f64), (0.0, im.height as f64)]
        .into_iter()
        .map(|(x, y)| {
            let p = im.xf * kurbo::Point::new(x, y);
            json!({"p":[p.x,p.y]})
        })
        .collect();
    json!({"id":n.id.0,"type":"path","path":{"subpaths":[{"closed":true,"anchors":anchors}]},
        "fillRule":"nonzero","fills":[{"type":"image"}],"opaqueImage":true})
}

// Object attributes follow copies and clipboard operations without changing the file format.
fn metadata<'a>(n: &'a Node, key: &str) -> Option<&'a str> {
    let key = format!("{META}-{key}");
    n.extras().attrs.as_ref()?.data.iter().find(|(k, _)| k == &key).map(|(_, v)| v.as_str())
}

fn control_name(n: &Node) -> Option<&str> {
    if matches!(n.kind, NodeKind::Image(_)) {
        return None;
    }
    metadata(n, "rule").or(n.name.as_deref())
}

fn selected_colors(s: &Session, ids: &[NodeId]) -> Result<BTreeMap<String, vectorcraft_trapping::Color>> {
    let doc = &s.doc()?.doc;
    // A snapshot can show a selection containing unsupported objects; Apply explains the error.
    Ok(ids
        .iter()
        .filter_map(|id| doc.node(*id))
        .filter(|n| control_name(n).and_then(leer_ajuste).is_none())
        .filter_map(|n| encoded_color(doc, &n.appearance.fill_paint()).ok())
        .filter_map(|v| vectorcraft_trapping::Color::leer(&v))
        .map(|c| (vectorcraft_trapping::etiqueta(&c), c))
        .collect())
}

fn black_labels(s: &Session, ids: &[NodeId]) -> Result<Vec<String>> {
    Ok(selected_colors(s, ids)?.into_iter().filter_map(|(key, color)| color.es_negro().then_some(key)).collect())
}

fn display_label(c: &vectorcraft_trapping::Color, key: &str) -> String {
    match c {
        vectorcraft_trapping::Color::Spot { name, tint, .. } => format!("{name} {:.1}%", tint * 100.0),
        _ => key.into(),
    }
}

/// Cheap read-only snapshot. Never calculates intersections during UI frames.
pub fn snapshot(s: &Session) -> Result<Value> {
    info(s, &json!({}))
}

fn info(s: &Session, p: &Value) -> Result<Value> {
    let st = s.doc()?;
    let ids = targets(s, p)?;
    let selected = ids_param(p, "ids").unwrap_or_else(|| st.selection.objects.clone());
    let blacks = black_labels(s, &ids)?;
    let colors = selected_colors(s, &ids)?;
    let label = |key: &str, id: NodeId, side: &str| {
        source_label(&st.doc, &colors, key).unwrap_or_else(|| st.doc.node(id).and_then(|n| metadata(n, side)).unwrap_or(key).to_string())
    };
    let mut pairs = Vec::new();
    let mut artwork = 0;
    let mut local_traps = Vec::new();
    let mut gradients = 0;
    let mut images = 0;
    for id in &ids {
        let Some(n) = st.doc.node(*id) else { continue };
        if let Some(record) = local::record(n) {
            local_traps.push(json!({"id":id,"name":n.display_name(),"record":record,"overprint":n.appearance.items.iter().any(|i|i.overprint())}));
            continue;
        }
        match control_name(n).and_then(leer_ajuste) {
            Some(Ok(Some(a))) => pairs.push(json!({
                "id": id.0, "name": n.display_name(), "left": label(&a.izq, *id, "left"), "right": label(&a.der, *id, "right"), "leftKey": a.izq, "rightKey": a.der,
                "relation": a.rel.to_string(), "widthMm": a.ancho_mm, "solidIntoGradient": a.der.starts_with("GL"), "solidIntoImage": a.der.starts_with("IM"),
                "blackLeft": blacks.contains(&a.izq), "blackRight": blacks.contains(&a.der),
            })),
            Some(_) => {}
            None => {
                artwork += 1;
                if matches!(n.kind,NodeKind::Image(_)) { images+=1; }
                if matches!(n.appearance.fill_paint(), Paint::Gradient(_)) { gradients += 1; }
            }
        }
    }
    Ok(
        json!({"ids": selected.iter().map(|id| id.0).collect::<Vec<_>>(), "artwork": artwork, "gradients": gradients, "images": images, "pairs": pairs, "localTraps":local_traps}),
    )
}

fn query(s: &mut Session, p: &Value) -> Result<Value> {
    info(s, p)
}

/// Unsupported art must fail before an edit, rather than produce plausible but wrong traps.
fn validate(s: &Session, ids: &[NodeId]) -> Result<()> {
    if ids.len() > MAX_INPUTS {
        return Err(bad(APPLY, "Selecciona como máximo 80 objetos, incluidos los traps. Trabaja por partes."));
    }
    let doc = &s.doc()?.doc;
    let mut count = 0;
    let mut artwork = 0;
    let mut checked_images = BTreeMap::new();
    let mut pixels = 0;
    let mut image_bytes = 0;
    for id in ids {
        let n = doc.node(*id).ok_or_else(|| bad(APPLY, "Un objeto ya no existe. Vuelve a seleccionarlo."))?;
        if local::is_local(n) {
            return Err(bad(
                APPLY,
                "Una franja local incorporada no se recalcula con Aplicar a la selección. Deshaz o elimina esa franja y vuelve a previsualizar.",
            ));
        }
        let control = control_name(n).and_then(leer_ajuste).is_some();
        if let Some((path, _, _)) = objects::geometry(n) {
            count += path.anchor_count();
            for (_, _, a) in path.anchors() {
                if [a.p, a.h_in, a.h_out].iter().any(|v| !v.x.is_finite() || !v.y.is_finite() || v.x.abs() > 4e6 || v.y.abs() > 4e6) {
                    return Err(bad(APPLY, "El trazado contiene coordenadas fuera de rango."));
                }
            }
        }
        if n.opacity != 1.0
            || n.blend != BlendMode::Normal
            || n.isolate
            || n.knockout == vectorcraft_doc::Knockout::On
            || n.extras().mask.is_some()
            || !n.appearance.effects.is_empty()
        {
            return Err(bad(APPLY, "El arte y los traps necesitan opacidad plena, mezcla normal y ausencia de máscaras o efectos."));
        }
        let mut parent = doc.parent_of(*id);
        while let Some(pid) = parent {
            let Some(p) = doc.node(pid) else { break };
            if p.isolate
                || p.knockout == vectorcraft_doc::Knockout::On
                || p.opacity != 1.0
                || p.blend != BlendMode::Normal
                || p.extras().mask.is_some()
                || !p.appearance.effects.is_empty()
                || p.appearance.items.iter().any(|i| !i.paint().is_none())
                || matches!(p.kind, NodeKind::Group { clip: true, .. } | NodeKind::Layer { clip: true, .. })
            {
                return Err(bad(
                    APPLY,
                    "Saca los trazados de grupos con apariencia, aislamiento, knockout, recorte, efectos o transparencia antes de aplicar trapping.",
                ));
            }
            parent = doc.parent_of(pid);
        }
        if control {
            continue;
        }
        artwork += 1;
        if let NodeKind::Image(im) = &n.kind {
            if n.extras().trace.is_some() || n.extras().perspective.is_some() {
                return Err(bad(APPLY, "Usa una imagen normal, sin calco ni perspectiva vinculada."));
            }
            if n.appearance.items.iter().any(|item| !item.paint().is_none()) {
                return Err(bad(APPLY, "La imagen debe estar sin rellenos ni contornos añadidos."));
            }
            validate_image(doc, im, &mut checked_images, &mut pixels, &mut image_bytes)?;
            count += 4;
            continue;
        }
        if matches!(n.kind, NodeKind::Compound { .. }) {
            let mut unsupported = false;
            n.walk(&mut |child| {
                if child.id != n.id && !matches!(child.kind, NodeKind::Path { .. }) {
                    unsupported = true;
                }
            });
            if unsupported {
                return Err(bad(APPLY, "El trazado compuesto contiene objetos que no son trazados."));
            }
        }
        let fills: Vec<_> = n
            .appearance
            .items
            .iter()
            .filter_map(|i| match i {
                AppearanceItem::Fill(f) => Some(f),
                _ => None,
            })
            .collect();
        let compatible = matches!(fills.as_slice(), [f]
            if f.visible && !f.overprint && f.opacity == 1.0 && f.blend == BlendMode::Normal && f.effects.is_empty());
        if compatible && let Some(f) = fills.first() {
            match &f.paint {
                Paint::Solid { .. } => {
                    encoded_color(doc, &f.paint)?;
                }
                Paint::Gradient(_) => validate_gradient(doc, &f.paint)?,
                _ => return Err(bad(APPLY, "Usa un relleno sólido o degradado lineal opaco.")),
            }
        }
        let strokes =
            n.appearance.items.iter().any(|i| matches!(i, AppearanceItem::Stroke(st) if st.width > 0.0 && !matches!(st.paint, Paint::None)));
        if !compatible || strokes || n.opacity != 1.0 || n.blend != BlendMode::Normal || !n.appearance.effects.is_empty() || n.extras().mask.is_some()
        {
            return Err(bad(APPLY, "Usa un relleno sólido o degradado lineal opaco, sin contorno, efectos, sobreimpresión previa ni transparencia."));
        }
    }
    if count > MAX_POINTS {
        return Err(bad(APPLY, "La selección supera 12.000 nodos. Simplifica o trabaja por partes."));
    }
    if artwork < 2 {
        return Err(bad(APPLY, "Selecciona dos objetos compatibles, por ejemplo un sólido y una imagen opaca; incluye sus traps al recalcular."));
    }
    Ok(())
}

fn prepare(s: &Session, p: &Value) -> Result<(Opciones, Vec<NodeId>, Value)> {
    let opc = options(p)?;
    let ids = targets(s, p)?;
    let st = s.doc()?;
    let selected = ids_param(p, "ids").unwrap_or_else(|| st.selection.objects.clone());
    for id in &selected {
        let Some(node) = st.doc.node(*id) else {
            return Err(bad(APPLY, "Un objeto seleccionado ya no existe."));
        };
        let mut unsupported = false;
        node.walk(&mut |n| {
            if n.visible
                && !n.locked
                && !matches!(
                    n.kind,
                    NodeKind::Group { .. } | NodeKind::Layer { .. } | NodeKind::Path { .. } | NodeKind::Compound { .. } | NodeKind::Image(_)
                )
            {
                unsupported = true;
            }
        });
        if unsupported {
            return Err(bad(
                APPLY,
                "La selección contiene textos u objetos vivos. Usa sólidos, degradados lineales opacos o imágenes PNG/JPEG opacas incrustadas.",
            ));
        }
    }
    validate(s, &ids)?;
    let st = s.doc()?;
    let blacks = black_labels(s, &ids)?;
    let objects = ids
        .iter()
        .map(|id| {
            let n = st.doc.node(*id).ok_or_else(|| bad(APPLY, "El objeto ya no existe."))?;
            if let NodeKind::Image(im) = &n.kind {
                Ok(image_input(n, im))
            } else {
                objects::encode(n).ok_or_else(|| bad(APPLY, "El objeto no tiene geometría compatible."))
            }
        })
        .collect::<Result<Vec<Value>>>()?;
    let mut input = json!({"objects":objects});
    input["nativeInks"] = json!(true);
    for o in input.get_mut("objects").and_then(Value::as_array_mut).into_iter().flatten() {
        let id = o.get("id").and_then(Value::as_u64).ok_or_else(|| bad(APPLY, "Objeto sin identidad."))?;
        let n = st.doc.node(NodeId(id)).ok_or_else(|| bad(APPLY, "El objeto ya no existe."))?;
        if matches!(n.kind, NodeKind::Image(_)) {
            continue;
        }
        if let Some(rule) = control_name(n).filter(|name| leer_ajuste(name).is_some()) {
            o["name"] = json!(rule);
            continue;
        }
        if matches!(n.appearance.fill_paint(), Paint::Gradient(_)) {
            // Host-only marker; the legacy plug-in contract and source paint are unchanged.
            o["opaqueLinear"] = json!(true);
        } else if let Some(fill) = o.get_mut("fills").and_then(Value::as_array_mut).and_then(|v| v.first_mut()) {
            fill["color"] = encoded_color(&st.doc, &n.appearance.fill_paint())?;
        }
    }
    let reset =
        p.get("resetExisting").map(|v| v.as_bool().ok_or_else(|| bad(APPLY, "resetExisting debe ser booleano."))).transpose()?.unwrap_or(false);
    if reset {
        for o in input.get_mut("objects").and_then(Value::as_array_mut).into_iter().flatten() {
            if o.get("name").and_then(Value::as_str).and_then(leer_ajuste).is_some() {
                o["name"] = json!("trap");
            }
        }
    }
    // Pair changes are made to the calculation input, never as a separate document edit.
    if let Some(pair) = p.get("pair") {
        let id = pair.get("id").and_then(Value::as_u64).ok_or_else(|| bad(APPLY, "pair.id es obligatorio."))?;
        let o = input
            .get_mut("objects")
            .and_then(Value::as_array_mut)
            .and_then(|v| v.iter_mut().find(|o| o["id"] == id))
            .ok_or_else(|| bad(APPLY, "Selecciona el arte y el trap del par que quieres cambiar."))?;
        let mut a = o
            .get("name")
            .and_then(Value::as_str)
            .and_then(leer_ajuste)
            .and_then(|a| a.ok())
            .flatten()
            .ok_or_else(|| bad(APPLY, "El objeto seleccionado no es un trap con ajuste."))?;
        a.ancho_mm = number(pair, "widthMm", 0.1, 0.01, 2.0)?;
        let mode = choice(pair, "mode", "forward")?;
        if [a.izq.as_str(), a.der.as_str()].iter().any(|s| s.starts_with("GL") || s.starts_with("IM")) && !matches!(mode, "forward" | "none") {
            return Err(bad(
                APPLY,
                "Sólido contra degradado o imagen conserva la dirección hacia ese objeto. Puedes cambiar el ancho o excluir el par.",
            ));
        }
        match mode {
            "forward" => a.rel = '>',
            "reverse" => {
                std::mem::swap(&mut a.izq, &mut a.der);
                a.rel = '>';
            }
            "center" => a.rel = '=',
            "none" => a.rel = '-',
            "overprint" if blacks.contains(&a.izq) || blacks.contains(&a.der) => {
                if blacks.contains(&a.der) {
                    std::mem::swap(&mut a.izq, &mut a.der);
                }
                a.rel = 'S';
            }
            _ => return Err(bad(APPLY, "Modo de par inválido; sobreimpresión solo admite negro K.")),
        }
        o["name"] = json!(a.nombre());
    }
    Ok((opc, ids, input))
}

fn analyze(s: &mut Session, p: &Value) -> Result<Value> {
    let (opc, ids, input) = prepare(s, p)?;
    let mut report = analizar_con(&input, &opc);
    if let Some(e) = report.get("error").and_then(Value::as_str) {
        return Err(bad("trapping.analyze", e));
    }
    let colors = selected_colors(s, &ids)?;
    let st = s.doc()?;
    for encounter in report.get_mut("encounters").and_then(Value::as_array_mut).into_iter().flatten() {
        for field in ["left", "right"] {
            let key = encounter[field].as_str().unwrap_or_default();
            let label = source_label(&st.doc, &colors, key).unwrap_or_else(|| key.to_string());
            encounter[field] = json!(label);
        }
        for (field, label) in [("bottomId", "bottomName"), ("topId", "topName")] {
            if let Some(n) = encounter[field].as_u64().and_then(|id| st.doc.node(NodeId(id))) {
                encounter[label] = json!(n.display_name());
            }
        }
    }
    let selected = ids_param(p, "ids").unwrap_or_else(|| st.selection.objects.clone());
    report["ids"] = json!(selected.iter().map(|id| id.0).collect::<Vec<_>>());
    Ok(report)
}

fn apply(s: &mut Session, p: &Value) -> Result<Value> {
    let (opc, ids, input) = prepare(s, p)?;
    let parent = s.doc()?.insertion_parent();
    let value = atrapar_con(&input, &opc);
    if let Some(error) = value.get("error").and_then(Value::as_str) {
        return Err(bad(APPLY, error));
    }
    // Native-only appearance plan. The plug-in adapter remains unable to create swatch links.
    let mut inks: BTreeMap<String, Vec<Paint>> = BTreeMap::new();
    for o in value.get("objects").and_then(Value::as_array).into_iter().flatten() {
        if let (Some(name), Some(fills)) = (o.get("name").and_then(Value::as_str), o.get("nativeFills").and_then(Value::as_array)) {
            let paints =
                fills.iter().map(|p| serde_json::from_value(p.clone()).map_err(|e| bad(APPLY, e.to_string()))).collect::<Result<Vec<Paint>>>()?;
            inks.insert(name.to_string(), paints);
        }
    }
    let out_bytes = serde_json::to_vec(&value).map_err(|e| bad(APPLY, e.to_string()))?;
    let output = objects::decode(&out_bytes).map_err(|e| bad(APPLY, e.to_string()))?;
    let colors = selected_colors(s, &ids)?;
    let doc = s.doc()?.doc.clone();
    let label = |key: &str| source_label(&doc, &colors, key).unwrap_or_else(|| key.to_string());
    let custom_names: BTreeMap<NodeId, String> = ids
        .iter()
        .filter_map(|id| {
            let n = doc.node(*id)?;
            let name = n.name.as_ref()?;
            let generated = metadata(n, "generatedName")?;
            (name != generated).then(|| (*id, name.clone()))
        })
        .collect();
    let result = s.edit("Trapping", |d, sel| {
        let result = objects::apply_output(d, &ids, output, parent).map_err(|e| bad(APPLY, e.to_string()))?;
        for id in &result {
            if let Some(n) = d.node_mut(*id)
                && !matches!(n.kind, NodeKind::Image(_))
                && let Some(paints) = n.name.as_ref().and_then(|name| inks.get(name))
            {
                let rule = n.name.clone().unwrap_or_default();
                if let Some(Ok(Some(a))) = leer_ajuste(&rule) {
                    let (left, right) = (label(&a.izq), label(&a.der));
                    let suffix = if a.rel == '-' { "sin trap".to_string() } else { format!("{:.2} mm", a.ancho_mm) };
                    let generated: String = format!("Trap: {left} / {right} ({suffix})").chars().take(240).collect();
                    n.name = Some(custom_names.get(id).cloned().unwrap_or_else(|| generated.clone()));
                    n.edit_attrs(|attrs| {
                        for (key, value) in [("rule", rule), ("left", left), ("right", right), ("generatedName", generated)] {
                            let key = format!("{META}-{key}");
                            if let Some((_, old)) = attrs.data.iter_mut().find(|(k, _)| k == &key) {
                                *old = value;
                            } else {
                                attrs.data.push((key, value));
                            }
                        }
                    });
                }
                n.appearance = Appearance::default().into();
                for paint in paints {
                    let mut fill = FillLayer::new(paint.clone());
                    fill.overprint = true;
                    n.appearance.items.push(AppearanceItem::Fill(fill));
                }
            }
        }
        sel.set(result.iter().copied());
        Ok(result)
    })?;
    let mut report = snapshot(s)?;
    report["ids"] = json!(result.iter().map(|id| id.0).collect::<Vec<_>>());
    Ok(report)
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "trapping.incorporateLocal",
            "Incorporate Previewed Trapping Band",
            [],
            None,
            "{previewId: opaque session ticket returned by trapping.previewLocal} → {status,id,layerId,source,receiver,widthMm,overprint}; commits the exact fresh prepared band in a managed top layer, preserving source paint/spot/tint; one undo step; rejects stale or consumed tickets, changed CMS, overlapping local pairs and unsuitable managed layers; selection becomes the trap",
            has_doc,
            local::incorporate
        ),
        cmd!(query "trapping.previewLocal", "Preview Local Trapping Band", [], None,
            "{ids:[source,receiver], region:[x0,y0,x1,y1] points, widthMm?:0.01..0.5 (0.1), zoneDpi?:36..600, clipToRegion?:bool (false)} → bounded temporary original-curve band and highlighted raster; opaque solid fills and linked spots only; sampled contact gating, visible receiver clipping; clipToRegion confines the band to a manually selected region; no document, history or selection edits and no Apply integration",has_doc,preview_local),
        cmd!(query "trapping.verifyCurves", "Verify Local Original Curves", [], None,
            "{ids:[two distinct source IDs], region:[x0,y0,x1,y1] in points} → {experimental, ids, region, status, testedSamples, visibleSamples, occludedSamples, gapSamples, unresolvedSamples, truncated, fragments, fragmentsTruncated, stepMm, probeMm, localObjects, warnings}; read-only bounded numerical checks of original fill curves and clips, conservative opaque-solid visibility; not a full contact proof, trap decision or trap geometry",has_doc,verify_curves),
        cmd!(query "trapping.boundaries", "Detect Visible Ink Boundary Candidates", [], None,
            "{dpi?:36..600 (72), artboard?:index (0), region?:[x0,y0,x1,y1] in points, contrast?:5..100 (15 percent per plate), minPixels?:2..1000 (4)} → {experimental, dpi, pixelMm, region, width, height, plates, zoneCount, zones:[{index,pixels,bounds,plates,attribution}], sourceObjects, zoneMapBase64, zonesTruncated, markedPixels, discardedPixels, dataBase64, overlayBase64, warnings}; read-only candidates from sharp opposing ink changes in aligned composed plates, bounded conservative opaque-solid source attribution, no trap decisions or direction; max 1 MP output, 12 plates and 8 MP source images; first 200 zones reported, all retained marks shown", has_doc, boundary_capture),
        cmd!(query "trapping.capture", "Capture Experimental Ink Plate", [], None,
            "{plate?: Cyan|Magenta|Yellow|Black|spot swatch name, dpi?: 36..600 (150), artboard?: index (0), region?: [x0,y0,x1,y1] in document points} → {experimental, plate, plates, dpi, pixelMm, region, width, height, dataBase64, warnings}; read-only sampled plate preview of the composed document, including raster images; at most 4 MP output, 32 MP / 64 MiB source images; approximate overprint, no conflict detection or trapping; independent of selection and the 80-object Apply limit", has_doc, capture),
        cmd!(query "trapping.analyze", "Analyze Trapping Contacts", [], None,
            "{ids?, widthMm?, direction?, black?, difference?, resetExisting?, pair?} → {ids, encounters, pairCount}; read-only shared Apply plan for opaque flat fills, solid into opaque process linear gradients or embedded opaque PNG/JPEG image frames", has_doc, analyze),
        cmd!(query "trapping.info", "Trapping Info", [], None,
            "{ids?} → {ids, artwork, pairs: [{id, name, left, right, relation, widthMm}]}; no geometry calculation", has_doc, query),
        cmd!(
            "trapping.apply",
            "Apply Trapping",
            [],
            None,
            "{ids?, widthMm?: 0.01..2, direction?: auto|reverse|center, black?: overprint|trap, difference?: 0..100, resetExisting?: bool, pair?: {id, widthMm, mode: forward|reverse|center|none|overprint}} CMYK and named spot geometry as one undo step; solid into opaque process linear gradient uses only solid ink at constant width, overriding global direction/black/difference; native fills preserve spots and overprint; opaque embedded PNG/JPEG image frames accept only the solid ink at constant width; image/image, gradient/gradient, transparent or linked images and technical inks are not supported",
            has_doc,
            apply
        ),
    ]
}

fn capture(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "trapping.capture";
    let doc = &s.doc()?.doc;
    let plate = match p.get("plate") {
        None => "Cyan",
        Some(v) => v.as_str().ok_or_else(|| bad(C, "plate debe ser el nombre de una tinta."))?,
    };
    let dpi = match p.get("dpi") {
        None => 150.0,
        Some(v) => v.as_f64().ok_or_else(|| bad(C, "dpi debe ser numérico."))?,
    };
    let region = capture_region(doc, p, C)?;
    let mut renderer = vectorcraft_render::Renderer::new();
    let image = vectorcraft_render::technical::capture_plate(&mut renderer, doc, region, dpi, plate).map_err(|e| bad(C, e))?;
    let png = image.to_png().map_err(|e| bad(C, e))?;
    let plates: Vec<_> = vectorcraft_doc::inks::plates(doc).iter().map(|p| json!({"name":p.name,"spot":p.spot})).collect();
    Ok(json!({
        "experimental":true, "plate":plate, "plates":plates, "dpi":dpi, "pixelMm":25.4/dpi,
        "region":[region.x0,region.y0,region.x1,region.y1], "width":image.width,"height":image.height,
        "dataBase64":vectorcraft_format::base64_encode(&png),
        "colorSettings":vectorcraft_color::cms::active_settings(),
        "warnings":[
            "Captura experimental: negro = tinta; blanco = ausencia de tinta. No detecta conflictos ni genera traps.",
            "Muestreo limitado por los ppp: un detalle menor que un píxel puede perderse. No sirve para medir el ancho de trapping.",
            "Sobreimpresión aproximada mediante Multiply. Transparencias y mezclas siguen el renderizador de vista previa; no es una prueba RIP. Efectos y patrones requieren expandirse antes.",
            "Solo conserva spots que existan como muestras vinculadas en el documento. No recupera tintas planas convertidas a RGB o CMYK.",
            "Captura toda el área indicada, incluidas las capas imprimibles visibles bloqueadas y los traps existentes. No excluye automáticamente troqueles."
        ]
    }))
}

fn capture_region(doc: &Document, p: &Value, c: &str) -> Result<Rect> {
    if let Some(v) = p.get("region") {
        let a = v.as_array().filter(|a| a.len() == 4).ok_or_else(|| bad(c, "region debe contener [x0,y0,x1,y1]."))?;
        let values = a.iter().map(|v| v.as_f64().ok_or_else(|| bad(c, "Las coordenadas deben ser numéricas."))).collect::<Result<Vec<_>>>()?;
        let mut v = values.into_iter();
        let mut next = || v.next().ok_or_else(|| bad(c, "Falta una coordenada."));
        Ok(Rect::new(next()?, next()?, next()?, next()?))
    } else {
        let index = match p.get("artboard") {
            None => 0,
            Some(v) => v.as_u64().and_then(|i| usize::try_from(i).ok()).ok_or_else(|| bad(c, "artboard debe ser un índice válido."))?,
        };
        Ok(doc.artboards.get(index).ok_or_else(|| bad(c, "La mesa de trabajo no existe."))?.rect)
    }
}

fn boundary_capture(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "trapping.boundaries";
    let doc = &s.doc()?.doc;
    let read_number = |key: &str, default: f64| match p.get(key) {
        None => Ok(default),
        Some(v) => v.as_f64().ok_or_else(|| bad(C, format!("{key} debe ser numérico."))),
    };
    let dpi = read_number("dpi", 72.0)?;
    let contrast = read_number("contrast", 15.0)?;
    let min_pixels = match p.get("minPixels") {
        None => 4,
        Some(v) => v.as_u64().and_then(|v| usize::try_from(v).ok()).ok_or_else(|| bad(C, "minPixels debe ser un entero positivo."))?,
    };
    let region = capture_region(doc, p, C)?;
    let mut renderer = vectorcraft_render::Renderer::new();
    let result = vectorcraft_render::technical::capture_boundaries(&mut renderer, doc, region, dpi, contrast, min_pixels).map_err(|e| bad(C, e))?;
    let zones: Vec<_> = result
        .analysis
        .zones
        .iter()
        .enumerate()
        .map(|(i, z)| {
            let [x0, y0, x1, y1] = z.bounds.map(|v| f64::from(v) * 72.0 / dpi);
            let attribution = result.attribution.zones.get(i).map(|a| {
                let pairs: Vec<_> = a.pairs.iter().filter_map(|p| {
                    let left = result.attribution.sources.get(p.left)?;
                    let right = result.attribution.sources.get(p.right)?;
                    Some(json!({"ids":[left.id.0,right.id.0],"samples":p.samples}))
                }).collect();
                json!({"status":if pairs.is_empty(){"unresolved"}else if a.unresolved>0 || a.truncated{"partial"}else{"resolved"},
                    "pairs":pairs,"sampledProbes":a.sampled,"unresolvedProbes":a.unresolved,"pairsTruncated":a.truncated})
            });
            json!({"index":i+1,"pixels":z.pixels,"plates":z.plates,"bounds":[region.x0+x0,region.y0+y0,region.x0+x1,region.y0+y1],"attribution":attribution})
        })
        .collect();
    let plates: Vec<_> = result.plates.iter().map(|p| json!({"name":p.name,"spot":p.spot})).collect();
    let used: std::collections::BTreeSet<_> = result.attribution.zones.iter().flat_map(|z| z.pairs.iter().flat_map(|p| [p.left, p.right])).collect();
    let sources: Vec<_> = used
        .into_iter()
        .filter_map(|i| result.attribution.sources.get(i))
        .map(|o| {
            json!({"id":o.id.0,"name":o.name.chars().take(128).collect::<String>(),"plates":o.plates,
            "bounds":[o.bounds.x0,o.bounds.y0,o.bounds.x1,o.bounds.y1]})
        })
        .collect();
    let zone_map = vectorcraft_render::Rendered {
        width: result.composite.width,
        height: result.composite.height,
        pixels: result.analysis.zone_map.iter().flat_map(|z| [*z, 0, 0, 255]).collect(),
    };
    Ok(json!({
        "experimental":true,"dpi":dpi,"contrast":contrast,"minPixels":min_pixels,"pixelMm":25.4/dpi,
        "region":[region.x0,region.y0,region.x1,region.y1],"width":result.composite.width,"height":result.composite.height,
        "plates":plates,"zoneCount":result.analysis.zone_count,"zonesTruncated":result.analysis.zone_count>zones.len(),"zones":zones,
        "markedPixels":result.analysis.marked_pixels,"discardedPixels":result.analysis.discarded_pixels,
        "sourceObjects":sources,"sourceAttributionExperimental":true,
        "zoneMapBase64":vectorcraft_format::base64_encode(&zone_map.to_png().map_err(|e|bad(C,e))?),
        "dataBase64":vectorcraft_format::base64_encode(&result.composite.to_png().map_err(|e|bad(C,e))?),
        "overlayBase64":vectorcraft_format::base64_encode(&result.analysis.overlay.to_png().map_err(|e|bad(C,e))?),
        "colorSettings":vectorcraft_color::cms::active_settings(),
        "warnings":[
            "Candidatos para revisión: cambios bruscos donde aumenta una tinta y disminuye otra. No confirma necesidad, dirección ni ancho de trapping. Relaciona muestras con sólidos opacos cuando puede atribuir ambos lados.",
            "Ignora cambios suaves, tinta contra papel, adiciones de una sola tinta y fragmentos pequeños. Puede omitir contactos finos o producir falsos positivos en imágenes.",
            "El muestreo omite dos píxeles del borde de la captura; los detalles y huecos menores que la resolución pueden perderse. El grosor rojo y su halo son indicadores, no traps.",
            "Hereda la sobreimpresión aproximada y las restricciones de la captura experimental. No es una prueba RIP, no recupera spots convertidos y no excluye troqueles imprimibles ni traps existentes.",
            "La atribución usa un mapa temporal de identificadores y hasta 256 sondeos por zona; admite rellenos sólidos opacos sin trazo ni sobreimpresión. Los demás objetos bloquean conservadoramente su caja. Una zona puede reunir varios pares; no son contactos vectoriales confirmados.",
            "El informe detalla como máximo las primeras 200 zonas conectadas en orden de lectura. El mapa conserva todas las marcas retenidas. El cálculo es síncrono, sin progreso ni cancelación."
        ]
    }))
}

fn verify_curves(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "trapping.verifyCurves";
    let a = p["ids"].as_array().filter(|a| a.len() == 2).ok_or_else(|| bad(C, "ids debe contener exactamente dos objetos."))?;
    let read = |i: usize| a.get(i).and_then(Value::as_u64).map(NodeId).ok_or_else(|| bad(C, "Los IDs deben ser enteros positivos."));
    let ids = [read(0)?, read(1)?];
    if p.get("region").is_none_or(|v| !v.is_array()) {
        return Err(bad(C, "Indica la región de la zona que quieres verificar."));
    }
    let doc = &s.doc()?.doc;
    let region = capture_region(doc, p, C)?;
    let r = vectorcraft_render::curves::verify(doc, ids, region).map_err(|e| bad(C, e))?;
    Ok(json!({"experimental":true,"ids":ids,"region":[region.x0,region.y0,region.x1,region.y1],
        "status":if r.truncated || r.unresolved>0 {"incomplete"}else if r.visible>0 {"visibleBoundaryFound"}else{"noVerifiedBoundary"},
        "testedSamples":r.tested,"visibleSamples":r.visible,"occludedSamples":r.occluded,"gapSamples":r.gaps,"unresolvedSamples":r.unresolved,
        "truncated":r.truncated,"fragmentVerification":"midpointOnly","fragments":r.fragments,"fragmentsTruncated":r.fragments_truncated,"localObjects":r.local_objects,
        "stepMm":vectorcraft_render::curves::STEP_MM,"probeMm":vectorcraft_render::curves::PROBE_MM,
        "warnings":["Verificación numérica por muestreo de curvas originales y recortes, no una prueba de toda la frontera ni una decisión de trapping.",
            "Exige los dos sólidos a ambos lados a 0,0001 mm y 0,002 mm del contorno. Detalles entre sondeos o menores que la tolerancia pueden perderse.",
            "Imágenes, degradados, trazos, máscaras y sobreimpresión bloquean conservadoramente su caja. Un resultado incompleto no confirma ausencia de contacto.",
            "Los fragmentos verdes aproximan por puntos las curvas comprobadas; no representan el ancho de 0,10 mm ni son geometría nueva del documento."]}))
}

fn preview_local(s: &mut Session, p: &Value) -> Result<Value> {
    s.local_trap_preview = None;
    const C: &str = "trapping.previewLocal";
    let a = p["ids"].as_array().filter(|a| a.len() == 2).ok_or_else(|| bad(C, "Indica [origen, receptor]."))?;
    let read = |i| a.get(i).and_then(Value::as_u64).map(NodeId).ok_or_else(|| bad(C, "Los IDs deben ser enteros positivos."));
    let ids = [read(0)?, read(1)?];
    if p.get("region").is_none_or(|v| !v.is_array()) {
        return Err(bad(C, "Indica la región de la zona."));
    }
    let width = match p.get("widthMm") {
        None => 0.1,
        Some(v) => v.as_f64().ok_or_else(|| bad(C, "El ancho debe ser numérico."))?,
    };
    let doc = &s.doc()?.doc;
    let requested = capture_region(doc, p, C)?;
    let zone_dpi = match p.get("zoneDpi") {
        None => None,
        Some(v) => {
            Some(v.as_f64().filter(|d| d.is_finite() && (36.0..=600.0).contains(d)).ok_or_else(|| bad(C, "zoneDpi debe estar entre 36 y 600 ppp."))?)
        }
    };
    let exact = match p.get("clipToRegion") {
        None => false,
        Some(v) => v.as_bool().ok_or_else(|| bad(C, "clipToRegion debe ser verdadero o falso."))?,
    };
    let zone_dpi = if exact { None } else { zone_dpi };
    let map_halo = zone_dpi.map_or(0.0, |dpi| std::f64::consts::SQRT_2 * 72.0 / dpi + 0.05 * 72.0 / 25.4);
    let r = if exact {
        vectorcraft_render::trap_preview::preview_in_region(doc, ids, requested, width)
    } else {
        vectorcraft_render::trap_preview::preview_with_halo(doc, ids, requested, width, map_halo)
    }
    .map_err(|e| bad(C, e))?;
    let paint = doc
        .node(ids[0])
        .and_then(|n| {
            n.appearance
                .items
                .iter()
                .find(|i| i.visible() && i.opacity() > 0.0 && matches!(i, AppearanceItem::Fill(_)) && !matches!(i.paint(), Paint::None))
        })
        .map(|i| i.paint().clone())
        .ok_or_else(|| bad(C, "No se pudo leer la tinta de origen."))?;
    let b = r.band.to_bezpath().bounding_box();
    let mut report = json!({"experimental":true,"status":"previewOnly","ids":ids,"widthMm":width,
        "requestedRegion":[requested.x0,requested.y0,requested.x1,requested.y1],"clipToRegion":exact,
        "region":[r.region.x0,r.region.y0,r.region.x1,r.region.y1],"dpi":r.dpi,
        "width":r.base.width,"height":r.base.height,
        "dataBase64":vectorcraft_format::base64_encode(&r.base.to_png().map_err(|e|bad(C,e))?),
        "overlayBase64":vectorcraft_format::base64_encode(&r.overlay.to_png().map_err(|e|bad(C,e))?),
        "bandPath":r.band.to_bezpath().to_svg(),"bandBounds":[b.x0,b.y0,b.x1,b.y1],
        "bandAnchors":r.band.subpaths.iter().map(|s|s.anchors.len()).sum::<usize>(),
        "originalCurveRuns":r.runs.len(),"testedIntervals":r.tested,"checksPerInterval":5,"terminalChecksPerInterval":3,
        "zoneDpi":zone_dpi,"mapHaloPt":map_halo,"endpointTolerancePt":0.0001,"endpointMethod":"boundedCurveIntersections",
        "localObjects":r.local_objects,"sourcePaint":paint.clone(),"plannedOverprint":true,
        "warnings":["Previsualización temporal; no crea objetos ni sustituye Aplicar a la selección.",
        "Tramos originales separados en cruces geométricos locales y comprobados por hasta cinco sondeos por intervalo de hasta 0,05 mm; celdas terminales con sondeos interiores. Tolerancia de cruce 0,0001 puntos. No es una prueba continua de contacto; tangencias o detalles inciertos pueden omitirse.",
        "Franja de ancho nominal, uniones redondas y extremos rectos, recortada al receptor visible. En esquinas, extremos y zonas estrechas el área resultante depende del recorte.",
        "El resaltado indica geometría, no el color de sobreimpresión. Imágenes, degradados, máscaras y otras composiciones bloquean su caja; no es una prueba RIP.",
        "La región limita los tramos analizados; sus extremos de corte no son extremos completos del contacto. Hasta 32 superficies, 1.024 anclas de entrada y 4.096 anclas de salida."]});
    let ticket = local::remember(s, ids, requested, width, r.band, paint)?;
    report["previewId"] = json!(ticket);
    Ok(report)
}
