//! Shared-kit layered weathering and photographic presentation editors.
//! Numerical edits remain one render draft until the ordinary Save action.
use super::{ImageMethod, materials, studio};
use crate::{SolveApp, ui_kit};
use solidcraft_engine::render::job::looks::{LookKind, LookLayer};

#[derive(Clone, Debug, Default)]
pub(super) struct View {
    pub selected_aging: usize,
    pub selected_look: usize,
    pub(super) targets: Vec<materials::Target>,
    pub(super) search: String,
}
pub(super) fn note(app: &SolveApp, ui: &mut egui::Ui, key: &'static str) {
    ui_kit::text(ui, app.tr(key), ui_kit::Text::Note(key));
}
pub(super) fn number(app: &SolveApp, ui: &mut egui::Ui, key: &str, value: &mut f64, range: std::ops::RangeInclusive<f64>, speed: f64, suffix: &str) {
    let response = ui_kit::row(ui, app.tr(key), |ui| ui_kit::number_field(ui, value, range, speed, suffix));
    crate::scenario::publish_target(key, response.rect);
}
pub(super) fn failure(app: &mut SolveApp, result: Result<(), String>) {
    if let Err(error) = result {
        app.set_status(error, true);
    }
}
fn gpu(app: &SolveApp, ui: &mut egui::Ui) -> bool {
    if app.render_workspace.view.method != ImageMethod::GpuRaytrace {
        note(app, ui, "render.effects_gpu_only");
        return false;
    }
    true
}
pub(super) fn layer_actions(app: &SolveApp, ui: &mut egui::Ui, prefix: &str, selected: usize, length: usize) -> (i32, bool) {
    let mut movement = 0;
    let mut remove = false;
    ui.horizontal_wrapped(|ui| {
        let up = ui_kit::button(ui, app.tr("render.layer_up"), ui_kit::Button::Enabled(selected > 0));
        crate::scenario::publish_target(&format!("render:{prefix}-up"), up.rect);
        if up.clicked() {
            movement = -1;
        }
        let down = ui_kit::button(ui, app.tr("render.layer_down"), ui_kit::Button::Enabled(selected + 1 < length));
        crate::scenario::publish_target(&format!("render:{prefix}-down"), down.rect);
        if down.clicked() {
            movement = 1;
        }
        let delete = ui_kit::button(ui, app.tr("render.layer_remove"), ui_kit::Button::Enabled(length > 0));
        crate::scenario::publish_target(&format!("render:{prefix}-remove"), delete.rect);
        remove = delete.clicked();
    });
    (movement, remove)
}

pub(super) fn aging_controls(app: &mut SolveApp, ui: &mut egui::Ui) {
    super::wear::controls(app, ui);
    if app.render_workspace.view.method != ImageMethod::GpuRaytrace {
        return;
    }
    let title = app.tr("render.aging_airborne").to_string();
    ui_kit::section(ui, "render_aging_atmosphere", &title, false, |ui| {
        note(app, ui, "render.aging_airborne_help");
        super::environment::atmosphere(app, ui);
    });
}

pub(super) fn presentation_controls(app: &mut SolveApp, ui: &mut egui::Ui) {
    if !gpu(app, ui) {
        return;
    }
    studio::tone_controls(app, ui);
    note(app, ui, "render.effects_help");
    let mut editor = std::mem::take(&mut app.render_workspace.view.inspector.effects);
    let before = app.render_workspace.view.studio.finishing;
    let mut finish = before;
    let title = app.tr("render.look_title").to_string();
    ui_kit::section(ui, "render_creative_looks", &title, true, |ui| {
        let add = ui_kit::dropdown_menu(ui, "render_look_add", app.tr("render.look_add"), |ui| {
            for (index, kind) in LookKind::ALL.into_iter().enumerate() {
                if ui_kit::list_picker(ui, &format!("look-add-{index}"), app.tr(kind.key()), false).clicked() {
                    match finish.looks.push(LookLayer { kind, ..Default::default() }) {
                        Ok(()) => editor.selected_look = finish.looks.len() - 1,
                        Err(error) => app.set_status(error.to_string(), true),
                    }
                    ui.close();
                }
            }
        });
        crate::scenario::publish_target("render:look-add", add.rect);
        editor.selected_look = editor.selected_look.min(finish.looks.len().saturating_sub(1));
        for index in 0..finish.looks.len() {
            if let Some(layer) = finish.looks.get(index) {
                let label = format!("{} · {}", index + 1, app.tr(layer.kind.key()));
                let response = ui_kit::list_picker(ui, &format!("look-layer-{index}"), &label, index == editor.selected_look);
                crate::scenario::publish_target(&format!("render:look-layer:{index}"), response.rect);
                if response.clicked() {
                    editor.selected_look = index;
                }
            }
        }
        if !finish.looks.is_empty() {
            let (movement, remove) = layer_actions(app, ui, "look", editor.selected_look, finish.looks.len());
            if remove {
                failure(app, finish.looks.remove(editor.selected_look).map_err(|e| e.to_string()));
                editor.selected_look = editor.selected_look.min(finish.looks.len().saturating_sub(1));
            } else if movement != 0 {
                let next = if movement < 0 { editor.selected_look.saturating_sub(1) } else { editor.selected_look + 1 };
                failure(app, finish.looks.move_layer(editor.selected_look, next).map_err(|e| e.to_string()));
                editor.selected_look = next;
            }
        }
        if let Some(layer) = finish.looks.get_mut(editor.selected_look) {
            let response = ui_kit::checkbox(ui, app.tr("render.layer_enabled"), &mut layer.enabled);
            crate::scenario::publish_target("render:look-enabled", response.rect);
            number(app, ui, "render.look_strength", &mut layer.strength, 0.0..=1.0, 0.01, "");
        }
    });
    let title = app.tr("render.lens_effects_title").to_string();
    ui_kit::section(ui, "render_photographic_effects", &title, true, |ui| {
        let photo = &mut finish.photographic;
        for key in [
            "render.flare_strength",
            "render.flare_threshold",
            "render.flare_halo",
            "render.flare_streak",
            "render.chromatic_offset",
            "render.radial_distortion",
            "render.soft_focus",
            "render.sharpening",
            "render.film_grain",
        ] {
            let (field, range, speed, suffix) = match key {
                "render.flare_strength" => (&mut photo.flare.strength, 0.0..=2.0, 0.01, ""),
                "render.flare_threshold" => (&mut photo.flare.threshold, 0.001..=1e6, 0.05, ""),
                "render.flare_halo" => (&mut photo.flare.halo, 0.0..=2.0, 0.01, ""),
                "render.flare_streak" => (&mut photo.flare.streak, 0.0..=2.0, 0.01, ""),
                "render.chromatic_offset" => (&mut photo.chromatic_pixels, 0.0..=12.0, 0.1, " px"),
                "render.radial_distortion" => (&mut photo.radial_distortion, -0.3..=0.3, 0.005, ""),
                "render.soft_focus" => (&mut photo.soft_focus, 0.0..=1.0, 0.01, ""),
                "render.sharpening" => (&mut photo.sharpening, 0.0..=1.0, 0.01, ""),
                _ => (&mut photo.grain, 0.0..=0.15, 0.001, ""),
            };
            number(app, ui, key, field, range, speed, suffix);
        }
        let response = ui_kit::row(ui, app.tr("render.flare_ghosts"), |ui| ui_kit::number_field(ui, &mut photo.flare.ghosts, 0..=4, 1.0, ""));
        crate::scenario::publish_target("render:flare-ghosts", response.rect);
        let response = ui_kit::row(ui, app.tr("render.effects_seed"), |ui| ui_kit::number_field(ui, &mut photo.seed, 0..=u32::MAX, 1.0, ""));
        crate::scenario::publish_target("render:photo-seed", response.rect);
        note(app, ui, "render.lens_effects_help");
        if ui_kit::button(ui, app.tr("render.effects_reset_lens"), ui_kit::Button::Normal).clicked() {
            *photo = Default::default();
        }
    });
    if before != finish {
        app.render_workspace.view.studio.finishing = finish;
        app.render_workspace.view.dirty = true;
    }
    app.render_workspace.view.inspector.effects = editor;
    studio::post_controls(app, ui);
}
