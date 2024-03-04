use egui::{ViewportId, ViewportOutput};
pub use egui_winit;

use egui_winit::{winit::event_loop::EventLoopWindowTarget, EventResponse};
use winit::{event::WindowEvent, window::Window};

use super::painter::Painter;

// ----------------------------------------------------------------------------

/// Convenience wrapper for using [`egui`] from a [`glutin`] app.
pub struct EguiBackend {
    pub egui_ctx: egui::Context,
    pub egui_winit: egui_winit::State,
    pub painter: Painter,

    viewport_info: egui::ViewportInfo,

    // output from the last update:
    shapes: Vec<egui::epaint::ClippedShape>,
    pixels_per_point: f32,
    textures_delta: egui::TexturesDelta,
}

impl EguiBackend {
    pub fn new<E>(window: &Window, event_loop: &EventLoopWindowTarget<E>) -> Self {
        let painter = Painter::new();

        let pixels_per_point = window.scale_factor() as f32;

        let egui_ctx = egui::Context::default();
        let mut egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            ViewportId::ROOT,
            event_loop,
            Some(pixels_per_point),
            None,
        );
        egui_winit.set_max_texture_side(2048);

        Self {
            egui_ctx,
            egui_winit,
            painter,
            shapes: Default::default(),
            textures_delta: Default::default(),
            viewport_info: Default::default(),
            pixels_per_point,
        }
    }

    pub fn on_window_event(&mut self, window: &Window, event: &WindowEvent) -> EventResponse {
        self.egui_winit.on_window_event(window, event)
    }

    pub fn run(&mut self, window: &Window, run_ui: impl FnMut(&egui::Context)) {
        let raw_input = self.egui_winit.take_egui_input(window);
        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            viewport_output,
        } = self.egui_ctx.run(raw_input, run_ui);

        if viewport_output.len() > 1 {
            log::warn!("Multiple viewports not yet supported by egui-glutin-gl");
        }

        for (_, ViewportOutput { commands, .. }) in viewport_output {
            let mut screenshot_requested = false;
            egui_winit::process_viewport_commands(
                &self.egui_ctx,
                &mut self.viewport_info,
                commands,
                window,
                true,
                &mut screenshot_requested,
            );
            if screenshot_requested {
                log::warn!("Screenshot not yet supported by egui-glutin-gl");
            }
        }

        self.egui_winit
            .handle_platform_output(window, platform_output);

        self.shapes = shapes;
        self.pixels_per_point = pixels_per_point;
        self.textures_delta.append(textures_delta);
    }

    /// Paint the results of the last call to [`Self::run`].
    pub fn paint(&mut self, window: &Window) {
        let shapes = std::mem::take(&mut self.shapes);
        let mut textures_delta = std::mem::take(&mut self.textures_delta);

        for (id, image_delta) in textures_delta.set {
            self.painter.set_texture(id, &image_delta);
        }

        let pixels_per_point = self.pixels_per_point;

        let clipped_primitives = self.egui_ctx.tessellate(shapes, pixels_per_point);

        let dimensions: [u32; 2] = window.inner_size().into();
        self.painter
            .paint_primitives(dimensions, pixels_per_point, &clipped_primitives);

        for id in textures_delta.free.drain(..) {
            self.painter.free_texture(id);
        }
    }

    /// Call to release the allocated graphics resources.
    pub fn destroy(&mut self) {
        self.painter.destroy();
    }
}
