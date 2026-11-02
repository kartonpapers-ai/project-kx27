# Architecture

> Project KX27 · 2026-11-02 (KX27.8): the development workflow can omit an explicitly selected zone/pair without editing artwork, restore it for review, and select the next non-omitted pair. Current-capture decisions are visible in the pair selector and JSON diagnostic; changing the analysis clears them. Shared-ink rules remain on hold and upstream parity scores are unchanged. See [operator omissions](trapcolor-trapping.md#omitir-trapping-por-decisión-del-operador).

> Project KX27 · 2026-10-30 (KX27.7): a large modeless development window orders detection, zone/pair selection, curve verification, preview, incorporation and JSON diagnostics. Explicit pair context and a persistent primary action replace the long palette sequence for this workflow; original palette actions remain available after closing it. Shared caches, freshness guards and ink algorithms are unchanged; the shared-ink trial remains pending. Upstream parity scores are unchanged. See [guided development window](trapcolor-trapping.md#ventana-de-desarrollo-guiada).

> Project KX27 · 2026-10-29: a fresh local preview can be incorporated as its exact prepared vector band, with the original CMYK/linked-spot paint and overprint, in a managed top layer. A session ticket binds geometry to the document/CMS; atomic undo, consumed-ticket and overlapping-pair guards prevent accidental repetition. Native metadata identifies local traps separately from legacy Apply. Initial print-ink checks reject non-print sources and shared-plate reductions; sampling/RIP limitations and upstream parity scores are unchanged. See [incorporating a local band](trapcolor-trapping.md#incorporar-la-franja-revisada).

> Project KX27 · 2026-10-28: local trapping preview follows original Bézier intervals of an attributed opaque-solid pair, with explicit source/receiver direction and 0.10 mm default width. Five numerical checks per small interval gate temporary bands; visible receiver clipping, zoom, overlay comparison and fresh JSON geometry support review. No document edits or integration with Apply; its 80-object limit and upstream parity scores are unchanged. See [local band preview](trapcolor-trapping.md#previsualizar-una-franja-local).

> Project KX27 · 2026-10-27: attributed pairs can verify original fill/clip curves inside their candidate zone. Bounded numerical probes check conservative visible ownership, distinguish paper/occlusion/unknown results, show green sample markers and export fresh local diagnostics. Read-only: no offsets, trap generation or Apply-limit changes. Upstream parity scores are unchanged. See [local curve verification](trapcolor-trapping.md#verificar-curvas-locales).

> Project KX27 · 2026-10-26: visible boundary zones now carry bounded, conservative source-object pairs for opaque solid fills, including linked spots. A temporary hard-edge ID render respects occlusion and supported clips; unsupported compositing blocks attribution. Clicking a marked pixel or zone selects the original objects through `select.set`, with per-pair review and fresh JSON diagnostics. No trap geometry or Apply-limit changes; upstream parity scores are unchanged. See [source attribution](trapcolor-trapping.md#relacionar-zonas-con-originales-fase-3-inicial).

> Project KX27 · 2026-10-26: the Trapping palette exports a versioned JSON diagnostic with controls, plates, bounded object samples and only fresh cached reports. Browser download/native save use app services; no geometry calculation, image payloads or document edits. Contact analysis also tracks colour-setting freshness. Upstream parity scores are unchanged. See [diagnostic scope](trapcolor-trapping.md#exportar-diagnóstico-json).

> Project KX27 · 2026-10-26: experimental visible-ink boundary candidates compare aligned process/spot captures, highlight sharp opposing ink changes and report connected review zones without editing artwork. Bounded sampling, smooth-ramp and small-fragment filtering, map/overlay toggle and zone review; no source-object attribution, trap decision, direction or vector generation. The 80-object Apply limit and upstream parity scores are unchanged. See [phase 2 scope](trapcolor-trapping.md#fronteras-visibles-fase-2).

> Project KX27 · 2026-10-25: experimental read-only ink captures compose a full artboard or region per CMYK/linked-spot plate, including embedded raster alpha and clips. Sampled preview with approximate overprint, bounded pixels and source assets; no contact detection or new trap geometry. The 80-object Apply limit and upstream parity scores are unchanged. See [capture scope](trapcolor-trapping.md#captura-experimental-por-tintas-fase-1).

> Project KX27 · 2026-10-25: first solid-into-image trapping for fully opaque embedded PNG/JPEG frames, using the actual affine corners and only the solid ink in overprint. Image nodes and encoded pixels are preserved; transparency, silhouettes, linked images and image/image or image/gradient contacts remain unsupported. Upstream parity scores and estimates are unchanged. See [trapping scope](trapcolor-trapping.md).

> Project KX27 · 2026-10-24: opaque process linear gradients accept a solid-into-gradient trap at fixed width using only the solid ink in overprint. Sources stay unchanged; local inversion, gradient/gradient, radial/transparent/spot gradients and production validation remain out of scope. Upstream parity scores and estimates are unchanged. See [trapping scope](trapcolor-trapping.md).

> Project KX27 · 2026-10-24: read-only contact analysis shares the Apply geometry plan, reports source pairs and rule reasons, and invalidates after document/rule changes. Per-edge overrides, images and production RIP validation remain pending. Upstream parity scores and estimates are unchanged. See [trapping scope](trapcolor-trapping.md).

> Project KX27 · 2026-10-23: initial vector spot trapping preserves linked swatches/tints and overprint fills, with native per-object rule metadata and PDF export regressions. Images and technical inks remain unsupported; RIP validation is pending. Upstream parity scores and estimates are unchanged. See [trapping scope](trapcolor-trapping.md).

> Trap Color fork update · 2026-10-23: persistent Trapping panel with steppa-trap 0.4 geometry, per-pair controls and transactional undo. See trapcolor-trapping.md for scope and remaining limits. Upstream parity scores and estimates are unchanged.

> **Last reviewed:** 2026-10-10 · **Last updated:** 2026-10-12 · **Change:** minor (desktop printing uses craft-print; Windows 7 fallback retained) · **Target:** Adobe Illustrator 2026 (30.x)

How VectorCraft is built today. The design rationale lives in the local, gitignored `plan/architecture.md`;
this file describes what is committed. Day-to-day rules (clean-room, never crash, quality gates) are in
[`AGENTS.md`](../AGENTS.md) and [development.md](development.md).

## Shape

A Cargo workspace of 23 library crates under `crates/`, three apps under `apps/` and `xtask`, ~354k lines of Rust,
pure Rust (no handwritten JS/TS). The same engine runs the desktop app, the browser build (WASM), the headless
CLI and the MCP server.

## Layers

`cargo xtask layers` (part of `cargo xtask ci`) enforces that a crate depends only on lower layers, and that
nothing below L6 depends on a UI toolkit (egui, eframe, winit, rfd). Table: `xtask/src/layers.rs`.

| Layer | Crates | Role |
|---|---|---|
| L0 | `geom`, `color`; `affinity` (standalone, no workspace deps) | curves and transforms (on `kurbo`), colour models, ICC and conversions; the Affinity reader |
| L1 | `doc` | the document model: artboards, layers, objects, appearance, text, symbols; pure data + serde, structural sharing |
| L2 | `pathops`, `brush`, `trace`, `text`, `effects`, `plugins`, `three-d` | booleans/Shape Builder/offset/outline; brushes; Image Trace; fonts, shaping (HarfRust, skrifa), bidi and layout; live effects and the pixel-filter pipeline; sandboxed WebAssembly plug-ins (wasmi); 3D surfaces of revolution, projection and shading (`three-d`, on `geom` only) |
| L3 | `render`, `svg`, `pdf`, `format`, `eps`, `cad`, `metafile` | document to pixels (`vello_cpu`, multithreaded); SVG, PDF, native JSON, EPS/PostScript (with its own interpreter), DXF, EMF/WMF readers and writers |
| L4 | `tools` | tool state machines: pointer events in, commands and overlays out (Pen, Direct Selection, smart guides…) |
| L5 | `engine` | sessions, history (unlimited undo by structural sharing), selection, the command registry (~710 commands), file I/O, the panic guard |
| L6 | `ui-egui`, `mcp` | the egui front end (menus, panels, dialogs, canvas, i18n); the MCP server (JSON-RPC over stdio) |
| L7 | `apps/vectorcraft`, `apps/vectorcraft-cli`, `apps/vectorcraft-web`, `xtask` | desktop app (eframe, wgpu), CLI (`run`, `convert`, `info`, `bench`, `perf`, `mcp`), browser runner (WebGPU, WebGL2 fallback), build tooling |

`testkit` holds test helpers (fixtures, generators, render comparison) and is only a dev-dependency.

## Everything is a command

Every user-visible action is a `CommandSpec` in `crates/engine/src/cmd/*` (id, label, menu path, shortcut,
params doc, `enabled`, `run`, journal flag); UI-only commands are `UI_COMMANDS` in
`crates/ui-egui/src/menus.rs`. Tools emit commands (begin, preview, commit). Menus, the command palette, Actions,
the control channel, the CLI and MCP all reach the same registry, so anything a user can do an agent can do.
`command.batch` runs several as one undo step.

## Rendering

The canvas is rasterized on the CPU by `vello_cpu` on worker threads, off the UI thread, with caches per object
and per effect; the GPU (wgpu: Metal, DX12, Vulkan/GL; WebGPU/WebGL2 on the web) only composites the result. Raster
effects are rendered offscreen per object and cached while panning. See
[hardware-parity.md](hardware-parity.md).

## Files

The native `.vectorcraft` format is versioned, compressed JSON with embedded images, saved atomically, readable
down to v1 and from the old `.drawcraft` name. The `FORMATS` table in `crates/engine/src/cmd/fileio/mod.rs`
lists every format read or written; importers are fuzzed (`crates/engine/tests/import_fuzz.rs`). See
[file-format-parity.md](file-format-parity.md).

Desktop printer discovery and PDF submission use `craft-print`, pinned to an immutable
craft-libs revision. The engine still owns sheet composition, copies, separations and colour
management. Submission runs on the existing background worker; its receipt or refusal
reaches the status bar, and Quit waits for pending submissions. Windows 7 retains WMI discovery
and the installed viewer's Print/PrintTo verb;
its target excludes the shared backend because Windows.Data.Pdf is unavailable there.

## Agent control

- **Control channel:** JSON lines on a local TCP port (`--control 7979`), with real egui pointer and keyboard
  injection and screenshots ([control-protocol.md](control-protocol.md)).
- **MCP:** `vectorcraft-cli mcp`, attached to a running app or headless ([mcp.md](mcp.md)).
- **Sandboxing:** `--automation-read-root` / `--automation-write-root` confine what agents read and write.

## Robustness

No panics in shipped code (workspace clippy lints ban `unwrap`, `expect`, `panic!` and friends outside tests);
`vectorcraft_engine::guard` catches a panic at every entry point and rolls the document back; Data Recovery
saves copies in the background. See [development.md](development.md#robustness-vectorcraft-never-crashes).

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-12 | minor | Desktop PDF spooler shared through craft-print; exact physical sizes, composed copies and Windows 7 fallback retained. Scores and remaining gaps unchanged. |
| 2026-10-10 | minor | Added the `three-d` crate |
| 2026-10-10 | major | First version, from the workspace, the layering table and the existing docs |
