use std::path::Path;
use std::sync::{Arc, Mutex};
use dioxus::html::{link, section, style};
use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
use dioxus_desktop::tao::platform::unix::WindowBuilderExtUnix;
use dioxus_desktop::tao::window::Icon;
use image::GenericImageView;
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


    let img = image::open(&Path::new("logo.png")).unwrap();
    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8().into_raw();
    let icon = Icon::from_rgba(rgba, width, height).unwrap();
    dioxus_desktop::launch_with_props(
        app,
        app_props_arc.clone(),
        Config::default().with_window(
            WindowBuilder::new()
                .with_window_icon(Some(icon))
                .with_title("File Search")
                .with_resizable(true)
                .with_inner_size(LogicalSize::new(800, 544))
                .with_transparent(true)
                .with_rgba_visual(true),
        ),
    );

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
        head {
            link { rel: "stylesheet", r#type: "text/css", href: "css2/bootstrap.css" }
            link { rel: "stylesheet", href: "https://fonts.googleapis.com/css?family=Poppins:400,700&display=swap" }
            link { rel: "stylesheet", href: "css2/styles.css" }
            // link { rel: "stylesheet", href: "css/responsive.css" }
        }
        body {
            div {
                class: "container",
                div {
                    class: "row",
                    div {
                        class: "col-1 d-flex justify-content-center align-items-center "
                    }
                    div {
                        class: "col-2 d-flex justify-content-center align-items-center",
                        div {
                            class: "menu-btn",
                            onclick: move |_| active_window.set(ActiveWindow::StartWindow), "Start Window"
                        }
                    }
                    div {
                        class: "col-2 d-flex justify-content-center align-items-center",
                        div {
                            class: "menu-btn",
                            onclick: move |_| active_window.set(ActiveWindow::FileSearch), "File Search"
                        }
                    }
                    div {
                        class: "col-2 d-flex justify-content-center align-items-center",
                        div {
                            class: "menu-btn",
                            onclick: move |_| active_window.set(ActiveWindow::FileIndex), "File Index"
                        }
                    }
                    div {
                        class: "col-2 d-flex justify-content-center align-items-center",
                        div {
                            class: "menu-btn",
                            onclick: move |_| active_window.set(ActiveWindow::ImageSearch), "Image Search"
                        }
                    }
                    div {
                        class: "col-2 d-flex justify-content-center align-items-center",
                        div {
                            class: "menu-btn",
                            onclick: move |_| active_window.set(ActiveWindow::ImageIndex), "Image Index"
                        }
                    }
                    div {
                        class: "col-1 d-flex justify-content-center align-items-center "
                    }
                }
            }
            match *active_window.get() {
                ActiveWindow::StartWindow => rsx! { start_window(cx) },
                ActiveWindow::FileSearch => rsx! { file_search(cx) },
                ActiveWindow::FileIndex => rsx! { file_index(cx) },
                ActiveWindow::ImageSearch => rsx! { image_search(cx) },
                ActiveWindow::ImageIndex => rsx! { image_index(cx) },
            }
        }
    })
}

fn start_window(cx: Scope<Arc<Mutex<App>>>) -> Element {
    cx.render(rsx! {
        div {
            class: "container centered",
            div {
                class: "row",
                div {
                    class: "col-md-6 d-flex justify-content-center align-items-center",
                    div {
                        class: "menu-btn1",
                        onclick: move |_| { enable_prefix_search(cx.props); }, "Enable Prefix Search"
                    }
                }
                div {
                    class: "col-md-6 d-flex justify-content-center align-items-center",
                    div {
                        class: "menu-btn1",
                        onclick: move |_| { enable_image_search(cx.props.clone()); }, "Enable Image Search"
                    }
                }
            }
        }
    })
}
