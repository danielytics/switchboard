use std::rc::Rc;

use crate::gui::window::Renderer;

pub enum TrayAction<S, R>
where
    S: AsRef<str>,
    R: Renderer,
{
    Quit,
    Callback { callback: Rc<dyn Fn() -> ()> },
    Window { title: S, renderer: R },
}

pub struct TrayItem<S, R>
where
    S: AsRef<str>,
    R: Renderer,
{
    pub label: S,
    pub enabled: bool,
    pub action: TrayAction<S, R>,
}
