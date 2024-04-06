use std::{fs, io};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::sync::{Arc, Mutex, Once};
use dioxus::html::button;
use dioxus::prelude::*;
use file_system::{dir_walker};
use trie::arc_str::ArcStr;
// use trie::arc_str::ArcStr;
use crate::dir_walker::DirWalker;


// fn main() -> io::Result<()> {

// }

// fn serialize(trie: &Trie) -> io::Result<()> {
//     let encoded: Vec<u8> = bincode::serialize(trie).unwrap();
//     fs::write("trie.bin", encoded)
// }
//
// fn deserialize() -> bincode::Result<Trie> {
//     let data = fs::read("trie.bin").unwrap();
//     bincode::deserialize(&data)
// }


// import the prelude to get access to the `rsx!` macro and the `Scope` and `Element` types


// use std::error::Error;
// use std::sync::{Arc, Mutex};
// use serde_json::Value;
// use image_to_text::{apply_for_caption, apply_for_labels};
// #[tokio::main]
// async fn main() {
//     let dir = r"./photos_test";
//
//     let callback_for_labels = Arc::new(Mutex::new(|response: Value| -> Result<(), Box<dyn Error>> {
//         println!("Label: {:?}", response);
//         Ok(())
//     }));
//
//     apply_for_labels(dir, callback_for_labels).await.unwrap();
//
//     let callback_for_captions = Arc::new(Mutex::new(|response: Value| -> Result<(), Box<dyn Error>> {
//         println!("Caption: {:?}", response);
//         Ok(())
//     }));
//
//     apply_for_caption(dir, callback_for_captions).await.unwrap();
// }

#[derive(Clone)]
enum ActiveWindow {
    StartWindow,
    FileSearch,
    FileIndex,
    ImageSearch,
    ImageIndex,
}

fn main() {
    // launch the dioxus app in a webview
    dioxus_desktop::launch(app);
}
fn initialize_map() -> Arc<Mutex<HashMap<ArcStr, Vec<ArcStr>>>> {
    let mut map: HashMap<ArcStr, Vec<ArcStr>> = HashMap::new();
    if let Ok(file) = File::open("map.bin") {
        let reader = BufReader::new(file);
        map = bincode::deserialize_from(reader).expect("Unable to deserialize map");
        println!("Map loaded");
    } else {
        println!("Map not loaded");
    };
    Arc::new(Mutex::new(map))
}
// define a component that renders a div with the four buttons
static INIT: Once = Once::new();
static mut MAP: Option<Arc<Mutex<HashMap<ArcStr, Vec<ArcStr>>>>> = None;

pub fn app(cx: Scope) -> Element {
    let active_window = use_state(cx, || ActiveWindow::StartWindow);



    unsafe {
        INIT.call_once(|| {
            MAP = Some(initialize_map());
        });
    }

    let map = unsafe { MAP.as_ref().unwrap().clone() };
    
    
    // let map = use_state(cx, || Arc::new(Mutex::new(HashMap::new())));
    cx.render(rsx! {
        div {
            button { onclick: move |_| active_window.set(ActiveWindow::FileSearch), "File Search" }
            button { onclick: move |_| active_window.set(ActiveWindow::FileIndex), "File Index" }
            button { onclick: move |_| active_window.set(ActiveWindow::ImageSearch), "Image Search" }
            button { onclick: move |_| active_window.set(ActiveWindow::ImageIndex), "Image Index" }
        }
        match *active_window.get() {
            ActiveWindow::StartWindow => rsx! { start_window(cx, active_window) },
            ActiveWindow::FileSearch => rsx! { file_search(cx, Arc::clone(&map)) },
            ActiveWindow::FileIndex => rsx! { file_index(cx, Arc::clone(&map)) },
            ActiveWindow::ImageSearch => rsx! { image_search(cx) },
            ActiveWindow::ImageIndex => rsx! { image_index(cx) },
        }
    })
}

fn start_window<'a>(cx: &'a Scoped<'a>, active_window: &'a UseState<ActiveWindow>) -> Element<'a> {
    cx.render(rsx! {
        div {
        }
    })
}

// define a component for each window

pub fn file_search<'a>(cx: &'a dioxus::prelude::Scoped<'a>, map: Arc<Mutex<HashMap<ArcStr, Vec<ArcStr>>>>) -> Element<'a> {
    let input_value = use_state(&cx, || "".to_string());
    let found_files = use_state(&cx, || Vec::new());
    let files: &Vec<String> = found_files.get();
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
                    let dir = input_value.get().clone();
                    let mut map = Arc::clone(&map);
                    let files = search_files(dir, &mut map);
                    found_files.set(files);
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

fn search_files(filename: String, map: &mut Arc<Mutex<HashMap<ArcStr, Vec<ArcStr>>>>) -> Vec<String> {
    println!("Searching for file: {}", filename);
    let mut files = Vec::new();
    if let Ok(map) = map.lock() {
        if let Some(paths) = map.get(&ArcStr(Arc::from(filename))) {
            for path in paths {
                println!("{}", path.0);
                files.push(path.0.to_string());
            }
        } else {
            println!("File not found");
        }
    }
    files
}

pub fn file_index<'a>(cx: &'a dioxus::prelude::Scoped<'a>, map: Arc<Mutex<HashMap<ArcStr, std::vec::Vec<ArcStr>>>>) -> Element<'a> {
    let input_value = use_state(&cx, || "".to_string());

    cx.render(rsx! {
        div {
            h1 { "Окно индексации файлов" }
            input {
                value: "{input_value}",
                oninput: move |event| {
                    let input = &event.value;
                    input_value.set(input.to_string());
                }
            }
            button {
                onclick: move |_| {
                    let dir = input_value.get().clone();
                    let mut map = Arc::clone(&map);
                    index_directory(dir, &mut map);
                },
                "Индексировать директорию"
            }
        }
    })
}

fn index_directory(dir: String, map: &mut Arc<Mutex<HashMap<ArcStr, Vec<ArcStr>>>>) {
    println!("Indexing directory: {}", dir);
    let map_for_closure = Arc::clone(map);
    if let Ok(mut it) = DirWalker::new(&dir) {
        it.walk_apply(move |path| {
            let path_string = Path::new(&path);
            let filename = path_string.file_name().unwrap().to_str().unwrap().to_string();
            let mut map = map_for_closure.lock().unwrap();
            if !map.contains_key(&ArcStr(Arc::from(filename.clone()))) {
                let mut v = Vec::new();
                v.push(ArcStr(Arc::from(path.to_string())));
                map.insert(ArcStr(Arc::from(filename.clone())), v);
            } else {
                map.get_mut(&ArcStr(Arc::from(filename.clone()))).unwrap().push(ArcStr(Arc::from(path.to_string())));
            }
        });
        println!("Indexing complete");
        let file = File::create("map.bin").expect("Unable to create file");
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, &*map.lock().unwrap()).expect("Unable to serialize map");
    }
}

pub fn image_search(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            h1 { "Image Search Window" }
        }
    })
}

pub fn image_index(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            h1 { "Image Index Window" }
            // Add your content here
        }
    })
}
