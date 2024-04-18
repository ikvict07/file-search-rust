use std::{fs::File, io::{BufReader, BufWriter}, path::Path};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};
use dioxus::html::{button, div};
use dioxus::prelude::*;
use dioxus_desktop::Config;
use file_system::{dir_walker};
use serde_json::Value;
use std::error::Error;
use app_props::app::*;

use dir_walker::DirWalker;
use std::cell::{RefCell, RefMut}; // Important we rewrite import from dioxus
use std::cmp::Ordering;
use std::io::ErrorKind;
use std::path::PathBuf;
use dioxus_desktop::wry::webview::Url;
use manganis::mg;
use trie_rs::{TrieBuilder, Trie};

use ui_facade::{file_index, image_index, image_search, on_click_file_search};
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
    if let Ok(mut app_props_guard) = app_props_arc.lock() {
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
            ActiveWindow::StartWindow => rsx! { start_window(cx, active_window) },
            ActiveWindow::FileSearch => rsx! { file_search(cx) },
            ActiveWindow::FileIndex => rsx! { file_index(cx) },
            ActiveWindow::ImageSearch => rsx! { image_search(cx) },
            ActiveWindow::ImageIndex => rsx! { image_index(cx) },
        }
    })
}

fn start_window<'a>(cx: Scope<'a, Arc<Mutex<App>>>, active_window: &'a UseState<ActiveWindow>) -> Element<'a> {
    cx.render(rsx! {
        div {
            h1 { "Start Window" }
            button { onclick: move |_| { enable_prefix_search(cx.props); }, "Enable prefix search" }
            button { onclick: move |_| { initialize_embeddings(cx.props.clone()); }, "Enable image search" }
        }
    })
}


pub fn file_search(cx: Scope<Arc<Mutex<App>>>) -> Element {
    let input_value = use_state(&cx, || "".to_string());
    let found_files: &UseState<Vec<String>> = use_state(&cx, || Vec::new());
    let results_state: &UseState<Vec<(String, u32, f32)>> = use_state(&cx, || Vec::new()); //uselles

    let files: &Vec<String> = found_files.get();

    let app = cx.props.lock().unwrap();

    cx.render(rsx! {
        div {
            h1 { "File Search Window" }
            input {
                value: "{input_value}",
                oninput: move |event| {
                    let input = &event.value;
                    input_value.set(input.to_string());
                }
            }
            button {
                onclick: move |_| {
                    let results= on_click_file_search(input_value.get().clone(), cx.props);
                    found_files.set(results);
                },
                "Поиск файлов"
            }
            div {
                for file in files {
                    div { file.clone() }
                }
            }
        }
    })
}







