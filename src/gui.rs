use anyhow::Result;
use std::rc::Rc;

use egui::Ui;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::gui::application::Application;
use crate::gui::trayicon::{TrayAction, TrayItem};
use crate::gui::window::Renderer;

pub mod application;
pub mod trayicon;
pub mod window;

pub enum Command {
    Quit,
    ReloadSettings,
}

struct UI {
    cmd_tx: Sender<Command>,
}

impl Renderer for UI {
    fn render(&mut self, ui: &mut Ui, close: &mut bool) {
        ui.heading("Settings");
        if ui.button("Close").clicked() {
            let _ = self.cmd_tx.send(Command::ReloadSettings);
            *close = true;
        }
    }
}

pub struct GUI {
    app: Application<&'static str, UI>,
    cmd_tx: Sender<Command>,
}

impl GUI {
    pub fn exit_fn(&self) -> Result<impl FnOnce() + 'static> {
        self.app.exit_fn()
    }

    pub async fn run(self) {
        self.app.run();
        println!("Event loop has exited");
        let _ = self.cmd_tx.send(Command::Quit).await;
    }
}

pub fn init() -> (GUI, Receiver<Command>) {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<Command>(10);
    let gui = GUI {
        cmd_tx: cmd_tx.clone(),
        app: Application::new(
            include_bytes!("../resources/icon.png"),
            [
                TrayItem {
                    label: "Settings",
                    enabled: true,
                    action: TrayAction::Window {
                        title: "Settings",
                        renderer: UI { cmd_tx: cmd_tx },
                    },
                },
                TrayItem {
                    label: "Test",
                    enabled: false,
                    action: TrayAction::Callback {
                        callback: Rc::new(|| {
                            println!("Hi");
                        }),
                    },
                },
                TrayItem {
                    label: "Quit",
                    enabled: true,
                    action: TrayAction::Quit,
                },
            ],
        ),
    };

    (gui, cmd_rx)
}
