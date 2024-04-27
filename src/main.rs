use std::sync::{Arc, Mutex};
use dioxus::prelude::*;
use dioxus_desktop::Config;
use app_props::app::*;


use ui_facade::{file_index, file_search, image_index, image_search};

#[derive(Clone)]
enum ActiveWindow {
    StartWindow,
    FileSearch,
    FileIndex,
    ImageSearch,
    ImageIndex,
}

#[tokio::main]
async fn main() {
    let app_props = App::new();
    let app_props_arc = Arc::new(Mutex::new(app_props));
    dioxus_desktop::launch_with_props(
        app,
        app_props_arc.clone(),
        Config::default());
    if let Ok(app_props_guard) = app_props_arc.lock() {
        if let Ok(mut db_guard) = app_props_guard.db.lock() {
            if let Some(db) = db_guard.take() {
                db.close();
            }
        }
    };
}

pub fn app(cx: Scope<Arc<Mutex<App>>>) -> Element {
    let active_window = use_state(cx, || ActiveWindow::StartWindow);

    cx.render(rsx! {
        div {
            button { onclick: move |_| active_window.set(ActiveWindow::StartWindow), "Start Window" }
            button { onclick: move |_| active_window.set(ActiveWindow::FileSearch), "File Search" }
            button { onclick: move |_| active_window.set(ActiveWindow::FileIndex), "File Index" }
            button { onclick: move |_| active_window.set(ActiveWindow::ImageSearch), "Image Search" }
            button { onclick: move |_| active_window.set(ActiveWindow::ImageIndex), "Image Index" }
        }
        match *active_window.get() {
            ActiveWindow::StartWindow => rsx! { start_window(cx) },
            ActiveWindow::FileSearch => rsx! { file_search(cx) },
            ActiveWindow::FileIndex => rsx! { file_index(cx) },
            ActiveWindow::ImageSearch => rsx! { image_search(cx) },
            ActiveWindow::ImageIndex => rsx! { image_index(cx) },
        }
    })
}

fn start_window<'a>(cx: Scope<'a, Arc<Mutex<App>>>) -> Element<'a> {
    cx.render(rsx! {
        div {
            h1 { "Start Window" }
            button { onclick: move |_| { enable_prefix_search(cx.props); }, "Enable prefix search" }
            button { onclick: move |_| { enable_image_search(cx.props.clone()); }, "Enable image search" }
        }
    })
}
