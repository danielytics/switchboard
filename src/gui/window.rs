use egui::Ui;
use std::sync::Arc;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::WindowAttributes};

pub trait Renderer {
    fn render(&mut self, ui: &mut Ui, close: &mut bool);
}

pub enum HandleEventResult {
    Handled,
    Closed,
}

pub struct Window<R: Renderer> {
    winit_window: Arc<winit::window::Window>,
    painter: egui_wgpu::winit::Painter,
    state: egui_winit::State,
    pub id: String,
    pub renderer: R,
}

impl<R: Renderer> Window<R> {
    pub fn new<S: AsRef<str>>(
        event_loop: &ActiveEventLoop,
        id: String,
        title: &S,
        renderer: R,
    ) -> Self {
        // Inside user_event when Settings is clicked:
        let attr = WindowAttributes::default().with_title(title.as_ref());
        let winit_window = Arc::new(event_loop.create_window(attr).unwrap());

        let painter_future = egui_wgpu::winit::Painter::new(
            egui::Context::default(),
            egui_wgpu::WgpuConfiguration::default(),
            false, // transparent
            egui_wgpu::RendererOptions::default(),
        );
        let mut painter = pollster::block_on(painter_future);

        let viewport_id = egui::viewport::ViewportId::ROOT;

        pollster::block_on(painter.set_window(viewport_id, Some(winit_window.clone())))
            .expect("Failed to assign window to painter");

        let egui_state = egui_winit::State::new(
            egui::Context::default(),
            viewport_id,
            &winit_window,
            Some(winit_window.scale_factor() as f32),
            None,
            None,
        );

        Self {
            winit_window,
            painter,
            state: egui_state,
            id,
            renderer,
        }
    }

    pub fn handle_event(&mut self, event: WindowEvent) -> HandleEventResult {
        let response = self.state.on_window_event(&self.winit_window, &event);
        if response.repaint {
            self.winit_window.request_redraw();
        }

        match event {
            WindowEvent::Resized(size) => {
                if let (Some(w), Some(h)) = (
                    std::num::NonZeroU32::new(size.width),
                    std::num::NonZeroU32::new(size.height),
                ) {
                    self.painter
                        .on_window_resized(egui::viewport::ViewportId::ROOT, w, h);
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let size = self.winit_window.inner_size();
                if let (Some(w), Some(h)) = (
                    std::num::NonZeroU32::new(size.width),
                    std::num::NonZeroU32::new(size.height),
                ) {
                    self.painter
                        .on_window_resized(egui::viewport::ViewportId::ROOT, w, h);
                }
            }
            WindowEvent::CloseRequested => {
                return HandleEventResult::Closed;
            }
            WindowEvent::RedrawRequested => {
                let window = &self.winit_window;
                let egui_ctx = self.state.egui_ctx().clone();

                // 1. Run egui frame
                let mut should_close = false;
                let full_output = egui_ctx.run(self.state.take_egui_input(window), |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| self.renderer.render(ui, &mut should_close));
                });
                if should_close {
                    return HandleEventResult::Closed;
                }

                // 2. Handle output
                self.state
                    .handle_platform_output(window, full_output.platform_output);

                // 3. Tessellate shapes
                let primitives =
                    egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

                // 4. Paint
                // This single call handles:
                // Texture updates, Buffer updates, Command encoding, and Presenting
                self.painter.paint_and_update_textures(
                    egui::viewport::ViewportId::ROOT,
                    full_output.pixels_per_point,
                    egui::Rgba::from_srgba_unmultiplied(30, 30, 30, 0).to_array(), // Clear color
                    &primitives,
                    &full_output.textures_delta,
                    Vec::new(),
                );
            }
            _ => (),
        }
        HandleEventResult::Handled
    }
}
