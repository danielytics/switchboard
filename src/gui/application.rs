use anyhow::Result;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent},
};
use winit::event_loop::EventLoop;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop,
    window::WindowId,
};

use crate::gui::trayicon::{TrayAction, TrayItem};
use crate::gui::window::{HandleEventResult, Renderer, Window};

#[derive(Debug)]
pub enum UserEvent {
    TrayIconEvent(TrayIconEvent),
    MenuEvent(MenuEvent),
    Exit,
}

struct MenuItemInfo<S>
where
    S: AsRef<str> + Send + 'static,
{
    id: String,
    label: S,
    enabled: bool,
}

enum Action<S, R>
where
    S: AsRef<str>,
    R: Renderer,
{
    Quit,
    Callback {
        callback: Rc<dyn Fn() -> ()>,
    },
    Window {
        title: S,
        renderer: RefCell<Option<R>>,
    },
}

pub struct Application<S, R>
where
    S: AsRef<str>,
    R: Renderer,
{
    event_loop: Option<EventLoop<UserEvent>>,
    _tray_icon: Option<TrayIcon>,
    window: Option<Window<R>>,
    actions: HashMap<String, Action<S, R>>,
}

impl<'a, S, R> Application<S, R>
where
    S: AsRef<str> + Send + 'static,
    R: Renderer,
{
    pub fn new<I>(icon_bytes: &[u8], menu: I) -> Self
    where
        I: IntoIterator<Item = TrayItem<S, R>>,
    {
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

        // Gather menu into renderers
        let mut actions = HashMap::new();
        let menu_items: Vec<MenuItemInfo<S>> = menu
            .into_iter()
            .enumerate()
            .map(|(idx, item)| {
                let id = idx.to_string();
                actions.insert(
                    id.clone(),
                    match item.action {
                        TrayAction::Quit => Action::Quit,
                        TrayAction::Callback { callback } => Action::Callback { callback },
                        TrayAction::Window { title, renderer } => Action::Window {
                            title,
                            renderer: RefCell::new(Some(renderer)),
                        },
                    },
                );
                MenuItemInfo {
                    id,
                    label: item.label,
                    enabled: item.enabled,
                }
            })
            .collect();

        let icon = load_icon(icon_bytes);

        #[cfg(target_os = "linux")]
        {
            std::thread::spawn(move || {
                gtk::init().expect("GTK init failed");
                let _tray = create_trayicon(icon, menu_items);
                gtk::main();
            });
        }

        Self {
            event_loop: Some(event_loop),
            #[cfg(not(target_os = "linux"))]
            _tray_icon: Some(Self::create_trayicon(icon, menu_items)),
            #[cfg(target_os = "linux")]
            _tray_icon: None,
            window: None,
            actions,
        }
    }

    pub fn exit_fn(&self) -> Result<impl FnOnce() + 'static> {
        if let Some(event_loop) = &self.event_loop {
            let proxy = event_loop.create_proxy();
            Ok(move || {
                let _ = proxy.send_event(UserEvent::Exit);
            })
        } else {
            anyhow::bail!("Event loop already started");
        }
    }

    pub fn run(mut self) {
        // Start the loop
        if let Some(event_loop) = self.event_loop.take() {
            event_loop.run_app(&mut self).unwrap()
        }
    }

    fn close_window(&mut self) {
        if let Some(window) = self.window.take() {
            match self.actions.get_mut(&window.id) {
                Some(Action::Window { renderer, .. }) => {
                    let r = renderer.get_mut();
                    if let None = r {
                        *r = Some(window.renderer);
                    }
                }
                _ => {}
            }
        }
    }
}

impl<S, R> ApplicationHandler<UserEvent> for Application<S, R>
where
    S: AsRef<str> + Send + 'static,
    R: Renderer,
{
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::MenuEvent(e) => {
                if let Some(action) = self.actions.get(&e.id.0) {
                    match action {
                        Action::Quit => {
                            println!("Quit action");
                            event_loop.exit();
                        }
                        Action::Window { title, renderer } => {
                            if let Some(renderer) = renderer.take() {
                                let _ = self
                                    .window
                                    .insert(Window::new(event_loop, e.id.0, title, renderer));
                            } else {
                                // If the window is already open, toggle it closed
                                self.close_window();
                            }
                        }
                        Action::Callback { callback } => callback(),
                    }
                }
            }
            UserEvent::Exit => {
                println!("Exit event");
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let closed = {
            let Some(window) = self.window.as_mut() else {
                return;
            };
            match window.handle_event(event) {
                HandleEventResult::Closed => true,
                _ => false,
            }
        };
        if closed {
            self.close_window();
        }
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}
}

fn load_icon(bytes: &[u8]) -> Icon {
    let image = image::load_from_memory(bytes).unwrap().into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).unwrap()
}

fn create_trayicon<S>(icon: Icon, items: Vec<MenuItemInfo<S>>) -> TrayIcon
where
    S: AsRef<str> + Send,
{
    let items: Vec<tray_icon::menu::MenuItem> = items
        .into_iter()
        .map(|i| tray_icon::menu::MenuItem::with_id(i.id, i.label, i.enabled, None))
        .collect();

    let item_refs: Vec<&dyn tray_icon::menu::IsMenuItem> = items
        .iter()
        .map(|i| i as &dyn tray_icon::menu::IsMenuItem)
        .collect();

    let tray_menu = Menu::new();
    // 3. This now works perfectly
    tray_menu.append_items(&item_refs).unwrap();

    TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_icon(icon)
        .build()
        .unwrap()
}
