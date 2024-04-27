use std::sync::{Arc, Mutex};
use dioxus::html::{link, section, style};
use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
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
        Config::default().with_window(
            WindowBuilder::new()
                .with_title("File Search")
                .with_resizable(false)
                .with_inner_size(LogicalSize::new(800, 544)),
        )
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
            link { rel: "stylesheet", r#type: "text/css", href: "css/bootstrap.css" }
            link { rel: "stylesheet", href: "https://fonts.googleapis.com/css?family=Poppins:400,700&display=swap" }
            link { rel: "stylesheet", href: "css/style.css" }
            link { rel: "stylesheet", href: "css/responsive.css" }
        }
        body {
            class: "sub_page",
            div {
                class: "hero_area",
                style: "display: flex; justify-content: center; align-items: center;",
                header {
                    class: "header_section",
                    div {
                        class: "container-fluid",
                        nav {
                            class: "navbar navbar-expand-lg custom_nav-container",
                            
                        }

                    }
                }
                section {
                    class: "experience_section layout_padding",
                    div {
                        class: "container",
                        div {
                            class: "row",
                            div {
                                class: "col-md-12",

                                div {
                                    class: "detail-box",
                                    div {
                                        class: "btn-box",
                                        a {
                                           // style: "margin-right: 10px;",
                                            class: "btn-0",
                                            onclick: move |_| active_window.set(ActiveWindow::StartWindow), "Start Window"
                                        }
                                        a {
                                           // style: "margin-right: 10px;",

                                            class: "btn-0",
                                            onclick: move |_| active_window.set(ActiveWindow::FileSearch), "File Search"
                                        }
                                        a {
                                          //  style: "margin-right: 10px;",

                                            class: "btn-0",
                                            onclick: move |_| active_window.set(ActiveWindow::FileIndex), "File Index"
                                        }
                                        a {
                                           // style: "margin-right: 10px;",

                                            class: "btn-0",
                                            onclick: move |_| active_window.set(ActiveWindow::ImageSearch), "Image Search"
                                        }
                                        a {
                                            class: "btn-0",
                                            onclick: move |_| active_window.set(ActiveWindow::ImageIndex), "Image Index"
                                        }
                                    }
                                }
                            }

                        }
                    }
                }
            }
            script {
                src: "js/jquery-3.4.1.min.js"
            }
            script {
                src: "js/bootstrap.js"
            }
            script {
                src: "js/custom.js"
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
            class: "hero_area",
            style: "display: flex; justify-content: center; align-items: center;",
            section {
                class: "experience_section layout_padding-top layout_padding2-bottom",
                div {
                    class: "container",
                    div {
                        class: "row",
                        div {
                            class: "col-md-12",
                            div {
                                class: "detail-box",
                                div {
                                    class: "btn-box",

                                    a {
                                        class: "btn-3",
                                        onclick: move |_| { enable_prefix_search(cx.props); }, "Enable Prefix Search"
                                    }
                                    a {
                                        class: "btn-3",
                                        onclick: move |_| { enable_image_search(cx.props.clone()); }, "Enable Image Search"
                                    }
                                }
                            }

                        }

                    }
                }
            }
        }
        // div {
        //     h1 { "Start Window" }
        //     button { onclick: move |_| { enable_prefix_search(cx.props); }, "Enable prefix search" }
        //     button { onclick: move |_| { enable_image_search(cx.props.clone()); }, "Enable image search" }
        // }
    })
}
