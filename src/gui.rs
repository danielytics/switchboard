use egui::Ui;
use std::sync::Arc;
use tokio::sync::mpsc;
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{WindowAttributes, WindowId},
};

const ID_SETTINGS: &str = "settings";
const ID_QUIT: &str = "quit";

pub enum Command {
    Quit,
    ReloadSettings,
}

#[derive(Debug)]
pub enum UserEvent {
    TrayIconEvent(TrayIconEvent),
    MenuEvent(MenuEvent),
}

struct WindowState {
    window: Arc<winit::window::Window>,
    painter: egui_wgpu::winit::Painter,
    state: egui_winit::State,
}

struct Application {
    // Tray icon
    _tray_icon: Option<TrayIcon>,
    // Settings window
    window_state: Option<WindowState>,
    // Channel for GUI to send events to main logic
    cmd_tx: mpsc::Sender<Command>,
    // Render function
    render_ui: Box<dyn Fn(&mut Ui, &mut bool)>,
}

impl ApplicationHandler<UserEvent> for Application {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        if let UserEvent::MenuEvent(e) = event {
            if e.id.0 == ID_QUIT {
                let _ = self.cmd_tx.try_send(Command::Quit);
                event_loop.exit();
            } else if e.id.0 == ID_SETTINGS {
                // If the window is already open, toggle it closed
                if !self.window_state.is_none() {
                    self.window_state = None;
                    return;
                }

                // Inside user_event when Settings is clicked:
                let attr = WindowAttributes::default().with_title("Settings");
                let window = Arc::new(event_loop.create_window(attr).unwrap());

                let painter_future = egui_wgpu::winit::Painter::new(
                    egui::Context::default(),
                    egui_wgpu::WgpuConfiguration::default(),
                    false, // transparent
                    egui_wgpu::RendererOptions::default(),
                );
                let mut painter = pollster::block_on(painter_future);

                let viewport_id = egui::viewport::ViewportId::ROOT;

                pollster::block_on(painter.set_window(viewport_id, Some(window.clone())))
                    .expect("Failed to assign window to painter");

                let egui_state = egui_winit::State::new(
                    egui::Context::default(),
                    viewport_id,
                    &window,
                    Some(window.scale_factor() as f32),
                    None,
                    None,
                );

                self.window_state = Some(WindowState {
                    window,
                    painter,
                    state: egui_state,
                });
            }
        }
    }
    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(ws) = &mut self.window_state else {
            return;
        };

        let response = ws.state.on_window_event(&ws.window, &event);
        if response.repaint {
            ws.window.request_redraw();
        }

        match event {
            WindowEvent::Resized(size) => {
                if let (Some(w), Some(h)) = (
                    std::num::NonZeroU32::new(size.width),
                    std::num::NonZeroU32::new(size.height),
                ) {
                    ws.painter
                        .on_window_resized(egui::viewport::ViewportId::ROOT, w, h);
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let size = ws.window.inner_size();
                if let (Some(w), Some(h)) = (
                    std::num::NonZeroU32::new(size.width),
                    std::num::NonZeroU32::new(size.height),
                ) {
                    ws.painter
                        .on_window_resized(egui::viewport::ViewportId::ROOT, w, h);
                }
            }
            WindowEvent::CloseRequested => {
                self.window_state = None;
            }
            WindowEvent::RedrawRequested => {
                let window = &ws.window;
                let egui_ctx = ws.state.egui_ctx().clone();

                // 1. Run egui frame
                let mut should_close = false;
                let full_output = egui_ctx.run(ws.state.take_egui_input(window), |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| (&self.render_ui)(ui, &mut should_close));
                });
                if should_close {
                    self.window_state = None;
                    return;
                }

                // 2. Handle output
                ws.state
                    .handle_platform_output(window, full_output.platform_output);

                // 3. Tessellate shapes
                let primitives =
                    egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

                // 4. Paint
                // This single call handles:
                // Texture updates, Buffer updates, Command encoding, and Presenting
                ws.painter.paint_and_update_textures(
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
    }
}

pub struct GUI {
    event_loop: EventLoop<UserEvent>,
    app: Application,
}

impl GUI {
    pub fn new(cmd_tx: mpsc::Sender<Command>) -> Self {
        let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();

        // Setup global event handlers
        let p_tray = event_loop.create_proxy();
        TrayIconEvent::set_event_handler(Some(move |e| {
            let _ = p_tray.send_event(UserEvent::TrayIconEvent(e));
        }));

        let p_menu = event_loop.create_proxy();
        MenuEvent::set_event_handler(Some(move |e| {
            let _ = p_menu.send_event(UserEvent::MenuEvent(e));
        }));

        #[cfg(target_os = "linux")]
        {
            std::thread::spawn(move || {
                gtk::init().expect("GTK init failed");
                let _tray = Self::create_trayicon();
                gtk::main();
            });
        }

        let app = Application {
            #[cfg(not(target_os = "linux"))]
            _tray_icon: Some(Self::create_trayicon()),
            #[cfg(target_os = "linux")]
            _tray_icon: None,
            window_state: None,
            cmd_tx,
            render_ui: Box::new(|ui, close| {
                ui.heading("Settings");
                if ui.button("Close").clicked() {
                    *close = true;
                }
            }),
        };

        Self { event_loop, app }
    }

    fn create_trayicon() -> TrayIcon {
        let tray_menu = Menu::new();
        let settings_i = MenuItem::with_id(ID_SETTINGS, "Settings", true, None);
        let quit_i = MenuItem::with_id(ID_QUIT, "Quit", true, None);
        tray_menu.append_items(&[&settings_i, &quit_i]).unwrap();

        TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_icon(load_icon())
            .build()
            .unwrap()
    }

    pub fn run(mut self) {
        // Start the loop
        self.event_loop.run_app(&mut self.app).unwrap();
    }
}

fn load_icon() -> Icon {
    let bytes = include_bytes!("../resources/icon.png");
    let image = image::load_from_memory(bytes).unwrap().into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).unwrap()
}
