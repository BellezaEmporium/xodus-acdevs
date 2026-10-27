use eframe::egui::{self, TextureHandle, TextureOptions};
use egui_extras::image::load_svg_bytes;
use std::path::Path;

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

pub(crate) fn load_svg_texture(
    ctx: &egui::Context,
    name: &str,
    path: impl AsRef<Path>,
) -> Option<TextureHandle> {
    let bytes = std::fs::read(path).ok()?;
    let color_image = load_svg_bytes(&bytes, &Default::default()).ok()?;
    Some(ctx.load_texture(name, color_image, TextureOptions::LINEAR))
}

pub(crate) fn load_texture(
    ctx: &egui::Context,
    name: &str,
    path: impl AsRef<Path>,
) -> Option<TextureHandle> {
    let bytes = std::fs::read(path).ok()?;

    let image = image::load_from_memory(&bytes).ok()?.to_rgba8();

    let size = [image.width() as usize, image.height() as usize];

    let pixels = image.into_raw();

    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);

    Some(ctx.load_texture(name, color_image, TextureOptions::LINEAR))
}
