use std::{fs, io};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard, Once};
use dioxus::html::button;
use dioxus::prelude::*;
use serde::{Deserialize, Deserializer, Serialize};
use file_system::{dir_walker};
use trie::arc_str::ArcStr;
// use trie::arc_str::ArcStr;
use crate::dir_walker::DirWalker;
use trie_rs::{Trie, TrieBuilder};
use once_cell::sync::Lazy;
use serde_json::Value;

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

fn initialize_map() -> Arc<Mutex<HashMap<ArcStr, HashSet<ArcStr>>>> {
    let mut map: HashMap<ArcStr, HashSet<ArcStr>> = HashMap::new();
    if let Ok(file) = File::open("map.bin") {
        let reader = BufReader::new(file);
        map = bincode::deserialize_from(reader).expect("Unable to deserialize map");
        println!("Map loaded");
    } else {
        println!("Map not loaded");
    };
    Arc::new(Mutex::new(map))
}

fn initialize_trie(map: &Arc<Mutex<HashMap<ArcStr, HashSet<ArcStr>>>>) -> Arc<Mutex<SomeTrie>> {
    let mut builder = TrieBuilder::new();
    let map = map.lock().unwrap();
    for key in map.keys() {
        // println!("pushing {} into trie with value {:?}", key.0.to_string(), value);
        builder.push(key.0.to_string());
    }
    let trie = builder.build();
    let trie = Arc::new(Mutex::new(SomeTrie::Trie(trie)));
    trie
}
// define a component that renders a div with the four buttons


// impl From<Trie<u8>> for SerializableTrie {
//     fn from(trie: Trie<u8>) -> Self {
//         let map = trie.;
//         SerializableTrie { map }
//     }
// }
// 
// impl From<SerializableTrie> for Trie<u8> {
//     fn from(s: SerializableTrie) -> Self {
//         s.map.into_iter().collect()
//     }
// }

pub enum SomeTrie {
    Trie(Trie<u8>),
    TrieBuilder(TrieBuilder<u8>),
}

static INIT: Once = Once::new();
static mut MAP: Option<Arc<Mutex<HashMap<ArcStr, HashSet<ArcStr>>>>> = None;
static mut TRIE: Option<Arc<Mutex<SomeTrie>>> = None;

static mut IS_PREFIX_SEARCH_ENABLED: Option<Arc<Mutex<bool>>> = None;

pub fn app(cx: Scope) -> Element {
    let active_window = use_state(cx, || ActiveWindow::StartWindow);


    unsafe {
        INIT.call_once(|| {
            MAP = Some(initialize_map());
            // TRIE = Some(initialize_trie(MAP.as_ref().unwrap()));
            IS_PREFIX_SEARCH_ENABLED = Some(Arc::new(Mutex::new(false)));
        });
    }

    let map = unsafe { MAP.as_ref().unwrap().clone() };
    let trie = unsafe {
        if let Some(t) = TRIE.as_ref() {
            t.clone()
        } else {
            Arc::new(Mutex::new(SomeTrie::TrieBuilder(TrieBuilder::new())))
        }
    };
    // let map = use_state(cx, || Arc::new(Mutex::new(HashMap::new())));
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
            ActiveWindow::FileSearch => rsx! { file_search(cx, Arc::clone(&map), Arc::clone(&trie)) },
            ActiveWindow::FileIndex => rsx! { file_index(cx, Arc::clone(&map), Arc::clone(&trie)) },
            ActiveWindow::ImageSearch => rsx! { image_search(cx) },
            ActiveWindow::ImageIndex => rsx! { image_index(cx) },
        }
    })
}

fn start_window<'a>(cx: &'a Scoped<'a>, active_window: &'a UseState<ActiveWindow>) -> Element<'a> {
    cx.render(rsx! {
        div {
            h1 { "Start Window" }
            button { onclick: move |_| { enable_prefix_search(); }, "Enable prefix search" }
        }
    })
}

// define a component for each window

fn enable_prefix_search() {
    unsafe {
        if let Some(is_enabled) = IS_PREFIX_SEARCH_ENABLED.as_ref() {
            if *is_enabled.lock().unwrap() {
            } else {
                println!("Initializing prefix search");
                TRIE = Some(initialize_trie(MAP.as_ref().unwrap()));
                IS_PREFIX_SEARCH_ENABLED = Some(Arc::from(Mutex::from(true)));
                println!("Prefix search enabled");
                println!("Trie: {:?}", IS_PREFIX_SEARCH_ENABLED.as_ref().unwrap().lock().unwrap().deref());
            }
        } else {

        }
    }
}

pub fn file_search<'a>(cx: &'a Scoped<'a>, map: Arc<Mutex<HashMap<ArcStr, HashSet<ArcStr>>>>, trie: Arc<Mutex<SomeTrie>>) -> Element<'a> {
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
                    // println!("Searching for files");
                    let dir = input_value.get().clone();
                    if let Some(is_enabled) = unsafe{IS_PREFIX_SEARCH_ENABLED.as_ref()} {
                        if *(is_enabled.lock().unwrap().deref()) {
                            if let SomeTrie::Trie(trie) = trie.lock().unwrap().deref() {
                                let found_names = trie.predictive_search(dir.clone().as_bytes());
                                let mut temp= Vec::new();
                                for (name) in found_names
                                    {
                                        let map_lock = map.lock().unwrap();
                                        for file in map_lock.get(&ArcStr(Arc::from(String::from_utf8(name.clone()).unwrap()))).unwrap() {
                                            temp.push(String::from_utf8(name.clone()).unwrap() + ": " + &*file.0.to_string());
                                        }
                                    }
                                found_files.set(temp);
                            }
                            else {

                            }
                        } else {
                            println!("im here!");
                            let map_lock = map.lock().unwrap();
                            let mut temp= Vec::new();
                            if let Some(files) = map_lock.get(&ArcStr(Arc::from(dir.clone()))) {
                                for file in files {
                                    temp.push(dir.clone() + ": " + &*file.0.to_string());
                                }
                            }
                            found_files.set(temp);
                        }
                    } else {

                    }
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


pub fn file_index<'a>(cx: &'a Scoped<'a>, mut map: Arc<Mutex<HashMap<ArcStr, HashSet<ArcStr>>>>, mut trie: Arc<Mutex<SomeTrie>>) -> Element<'a> {
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
                    index_directory(dir, &mut map, &mut trie);
                },
                "Индексировать директорию"
            }
        }
    })
}

fn index_directory(dir: String, map: &Arc<Mutex<HashMap<ArcStr, HashSet<ArcStr>>>>, trie: &mut Arc<Mutex<SomeTrie>>) {
    println!("Indexing directory: {}", dir);
    let map_for_closure = Arc::clone(map);
    if let Ok(mut it) = DirWalker::new(&dir) {
        it.walk_apply(move |path| {
            let path_string = Path::new(&path);
            let filename = path_string.file_name().unwrap().to_str().unwrap().to_string();
            let mut map = map_for_closure.lock().unwrap();
            if !map.contains_key(&ArcStr(Arc::from(filename.clone()))) {
                let mut v = HashSet::new();
                v.insert(ArcStr(Arc::from(path.to_string())));
                map.insert(ArcStr(Arc::from(filename.clone())), v);
            } else {
                map.get_mut(&ArcStr(Arc::from(filename.clone()))).unwrap().insert(ArcStr(Arc::from(path.to_string())));
            }
        });

        let file = File::create("map.bin").expect("Unable to create file");
        let writer = BufWriter::new(file);

        // let trie_ = build_trie(Arc::clone(map));
        //
        // let mut trie_guard = trie.lock().unwrap();
        // *trie_guard = trie_;

        bincode::serialize_into(writer, &*map.lock().unwrap()).expect("Unable to serialize map");
        println!("Directory indexed");
    }
}

fn build_trie(map: Arc<Mutex<HashMap<ArcStr, HashSet<ArcStr>>>>) -> SomeTrie {
    let mut builder = TrieBuilder::new();
    let map_lock = map.lock().unwrap();
    for (key, value) in map_lock.iter() {
        builder.push(key.0.to_string());
    }
    let trie_ = builder.build();
    SomeTrie::Trie(trie_)
}

pub fn image_search(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            h1 { "Image Search Window" }
            button {

            }
        }
    })
}

pub fn image_index(cx: Scope) -> Element {
    let input_value = use_state(&cx, || "".to_string());

    cx.render(rsx! {
        div {
            h1 { "Image index window" }
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
                    tokio::spawn(async move {
                        index_images(dir).await;
                    });
                },
                "Индексировать директорию"
            }
        }
    })
}
use std::error::Error;
use db::database::Save;
use image_to_text::{apply_for_caption, apply_for_labels};
use image_to_text::processor::ImageProcessor;

fn handle_response(response: Value) -> Result<(), Box<dyn Error>> {
    println!("Caption: {:?}", response);
    Ok(())
}
fn handle_response_label(response: Value) -> Result<(), Box<dyn Error>> {
    if let Some(responses) = response.get("responses") {
        if responses.is_array() {
            let responses = responses.as_array().unwrap();
            for (i, response) in responses.iter().enumerate() {
                println!("Response {}: ", i + 1);
                if let Some(label_annotations) = response.get("labelAnnotations") {
                    if label_annotations.is_array() {
                        let label_annotations = label_annotations.as_array().unwrap();
                        for (j, label_annotation) in label_annotations.iter().enumerate() {
                            let description = label_annotation.get("description").unwrap().as_str().unwrap();
                            let mid = label_annotation.get("mid").unwrap().as_str().unwrap();
                            let score = label_annotation.get("score").unwrap().as_f64().unwrap();
                            let topicality = label_annotation.get("topicality").unwrap().as_f64().unwrap();
                            println!("Label Annotation {}: Description: {}, Mid: {}, Score: {}, Topicality: {}", j + 1, description, mid, score, topicality);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
pub async fn index_images(dir: String) {
    // let secret = String::from("client_secret.json");
    // let label_processor = ImageProcessor::new_label(dir.clone(), secret.clone());
    // let caption_processor = ImageProcessor::new_caption(dir, secret, 3);
    // let label_callback = Arc::new(Mutex::new(handle_response_label));
    // let caption_callback = Arc::new(Mutex::new(handle_response));
    // let label_callback_clone = label_callback.clone();
    // let caption_callback_clone = caption_callback.clone();
    //
    // let label_task = tokio::spawn(async move {
    //     if let ImageProcessor::Label(label_processor) = label_processor.await {
    //         label_processor.process(label_callback_clone).await.unwrap();
    //     }
    // });
    // let caption_task = tokio::spawn(async move {
    //     if let ImageProcessor::Caption(caption_processor) = caption_processor.await {
    //         caption_processor.process(caption_callback_clone).await.unwrap();
    //     }
    // });
    // let _ = tokio::join!(label_task, caption_task);

    let db = db::database::Database::new();
    let vector = vec![2.0, 3.1, 4.3];
    let semantic_vector = db::semantic_vector::SemanticVec::from_vec(vector);

    let mut image = db::image::Image::new(String::from("path3"), String::from("title2"));
    image.set_semantic_vector(semantic_vector);
    if let Some(ref conn) = db.connection {
        image.save(conn);
    }

    let mut test = db.select_image_by_path("path3").unwrap();

    println!("Image: {:?}", test);

    db.close();
}