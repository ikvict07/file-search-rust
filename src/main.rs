use std::{fs::File, io::{BufReader, BufWriter}, path::Path};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};
use dioxus::html::{button, div};
use dioxus::prelude::*;
use dioxus_desktop::Config;
use serde::{Deserialize, Deserializer, Serialize};
use file_system::{dir_walker};
use trie_rs::{Trie, TrieBuilder};
use serde_json::Value;
use std::error::Error;

use trie::arc_str::ArcStr;
use dir_walker::DirWalker;
use db::{database::Save, image::Image};
use app_props::app::*;
use vectorization::*;
use std::cell::{RefCell, RefMut};
use std::cmp::Ordering;
use std::io::ErrorKind;
use std::path::PathBuf;
use dioxus_desktop::wry::webview::Url;
use manganis::mg;
use tokio::sync::Notify;
use db::semantic_vector::SemanticVec;
use img_azure::azure_api::{AzureRequest, AzureResponse, Label};

// Important we rewrite import from dioxus
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
    let map = initialize_map();
    let app_props = App {
        map: map,
        trie: Arc::new(Mutex::new(SomeTrie::TrieBuilder(TrieBuilder::new()))),
        is_prefix_search_enabled: Arc::new(Mutex::new(false)),
        embeddings: Arc::new(Mutex::new(Embedding::new())),
        db: Arc::new(Mutex::new(None)),
    };
    let app_props_arc = Arc::new(Mutex::new(app_props));
    dioxus_desktop::launch_with_props(
        app,
        app_props_arc.clone(),
        Config::default());

    println!("Hello, world!");
    // Try to lock the Mutex in App first
    if let Ok(mut app_props_guard) = app_props_arc.lock() {
        // Then try to lock the Mutex in db
        if let Ok(mut db_guard) = app_props_guard.db.lock() {
            // If both Mutexes are successfully locked, take and close the db
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
    let found_files:&UseState<Vec<String>> = use_state(&cx, || Vec::new());
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
                    let dir = input_value.get().clone();
                    let is_enabled = app.is_prefix_search_enabled.clone();
                    let mut temp= Vec::new();
                        if *(is_enabled.lock().unwrap()) { // Prefix search
                            if let SomeTrie::Trie(trie) = app.trie.lock().unwrap().deref() {
                                let found_names = trie.predictive_search(dir.clone().as_bytes());
                                for (name) in found_names {
                                    let map_lock = app.map.lock().unwrap();
                                    for file in map_lock.get(&ArcStr(Arc::from(String::from_utf8(name.clone()).unwrap()))).unwrap() {
                                        temp.push(String::from_utf8(name.clone()).unwrap() + ": " + &*file.0.to_string());
                                    }
                                }

                            }
                            else {
                                panic!("Trie is not initialized");
                            }
                        } else { // No prefix search
                            let map_lock = app.map.lock().unwrap();

                            if let Some(files) = map_lock.get(&ArcStr(Arc::from(dir.clone()))) {
                                for file in files {
                                    temp.push(dir.clone() + ": " + &*file.0.to_string());
                                }
                            }
                        }
                    found_files.set(temp);
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


pub fn file_index(cx: Scope<Arc<Mutex<App>>>) -> Element {
    let input_value = use_state(&cx, || "".to_string());
    let mut app = cx.props.lock().unwrap();
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
                    index_directory(dir, &mut *app);
                },
                "Индексировать директорию"
            }
        }
    })
}

fn index_directory(dir: String, app: &mut App) {
    println!("Indexing directory: {}", dir);
    let map_for_closure = Arc::clone(&app.map);
    if let Ok(mut it) = DirWalker::new(&dir) {
        it.walk_apply(move |path| {
            let path_string = Path::new(&path);
            let filename = path_string.file_name().unwrap().to_str().unwrap().to_string();
            let mut map = map_for_closure.lock().unwrap();
            let filename_arc: Arc<str> = Arc::from(filename.clone());
            let path_arc: Arc<str> = Arc::from(path);
            if !map.contains_key(&ArcStr(filename_arc.clone())) {
                let mut v = HashSet::new();
                v.insert(ArcStr(path_arc.clone()));
                map.insert(ArcStr(filename_arc.clone()), v);
            } else {
                map.get_mut(&ArcStr(filename_arc.clone())).unwrap().insert(ArcStr(path_arc.clone()));
            }
        });

        let file = File::create("map.bin").expect("Unable to create file");
        let writer = BufWriter::new(file);


        let mut map = app.map.lock().unwrap();
        if bincode::serialize_into(writer, &*map).is_err() {
            println!("Error serializing map");
        }
        println!("Directory indexed");
    }
}

pub fn image_search(cx: Scope<Arc<Mutex<App>>>) -> Element {
    let input_value = use_state(&cx, || "".to_string());
    let found_files:&UseState<Vec<String>> = use_state(&cx, || Vec::new()); //uselles here only to satisfy hook order
    let results_state: &UseState<Vec<(String, u32, f32)>> = use_state(&cx, || Vec::new());
    let app = cx.props.clone();

    cx.render(rsx! {
        div {
            h1 { "Image Search Window" }
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
                    let app_clone = app.clone();
                    let (tx, rx) = std::sync::mpsc::channel();

                    tokio::spawn(async move {
                        let results = search_images(dir, app_clone).await;
                        tx.send(results).unwrap();
                    });

                    let results = rx.recv().unwrap();
                    let mut r = Vec::new();
                    for (path, id, value) in results.iter() {
                        let src = format!("/{}", path);
                        r.push((src.clone(), *id, *value));
                    }
                    results_state.set(r.clone());
                },
                "Поиск изображений"
            }
            div {
                for (path, id, value) in results_state.get().iter() {
                    img {
                        src: &**path, // Передаем владеющий объект img_src, а не ссылку
                        width: "100",
                        height: "100"
                    }
                    div {
                        format!("Path: {}, Id: {}, Value: {}", path, id, value)
                    }
                }
            }
        }
    })
}


async fn search_images(dir: String, app: Arc<Mutex<App>>) -> Vec<(String, u32, f32)> {
    let embeddings = app.lock().unwrap().embeddings.clone();
    let db = app.lock().unwrap().db.clone();
    let mut db = db.lock().unwrap();
    let mut embeddings = embeddings.lock().unwrap();

    let res = embeddings.average_vector(dir.as_str());

    let ids = db.as_mut().unwrap().select_all_images();
    let mut results = Vec::new();
    let time = std::time::Instant::now();
    for id in ids {
        let mut vec = Vec::new();
        {
            let mut statement = db.as_mut().unwrap().connection.as_mut().unwrap().prepare("SELECT value FROM semantic_vectors WHERE image_id = ?1").unwrap();
            let mut rows = statement.query(&[&id]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                let value: f32 = row.get(0).unwrap();
                vec.push(value);
            }
        }
        {
            let mut statement = db.as_mut().unwrap().connection.as_mut().unwrap().prepare("SELECT path FROM images WHERE id = ?1").unwrap();
            let mut rows = statement.query(&[&id]).unwrap();
            let path: String = rows.next().unwrap().unwrap().get(0).unwrap();
            results.push((path, id, Embedding::cosine_similarity(&res, &vec)));
        }
    }

    results.sort_by(|a, b| match b.2.partial_cmp(&a.2) {
        None => {Ordering::Less}
        Some(res) => {res}
    }
    );
    for (path, id, value) in results.iter().take(10) {
        println!("Id: {}, Value: {}", id, value);
    }
    println!("Time: {:?}", time.elapsed());
    results.iter().take(10).map(|(path, id, value)| (path.clone(), *id, *value)).collect()
}

pub fn image_index(cx: Scope<Arc<Mutex<App>>>) -> Element {
    let input_value = use_state(&cx, || "".to_string());
    let app = cx.props.clone();
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
                    let app_clone = app.clone();
                    tokio::spawn(async move {
                        index_images(dir, app_clone).await;
                    });
                },
                "Индексировать директорию"
            }
        }
    })
}


pub async fn index_images<'a>(dir: String, app: Arc<Mutex<App>>) {
    let walker = DirWalker::new(&dir).unwrap();
    let embeddings = {
        let app = app.lock().unwrap();
        app.embeddings.clone()
    };
    let db = {
        let app = app.lock().unwrap();
        app.db.clone()
    };
    let db_for_closure = Arc::clone(&db);
    let db_for_send = db.clone();
    walker.send_requests_for_dir_apply(db_for_send,10, move|resp, path| {
        let mut db = db_for_closure.lock().unwrap();
        match resp {
            Ok(mut response) => {
                let mut label_vec = Vec::new();
                response.labels.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
                for label in response.labels.iter().take(10) {
                    label_vec.push(label.name.clone());
                }

                let semantic_vector = {
                    let mut semantic_vector_caption = (embeddings.lock().unwrap().average_vector(&response.caption));
                    let mut semantic_vector_labels = (embeddings.lock().unwrap().average_vector(&label_vec.join(" ")));
                    let mut v = Vec::new();
                    for i in 0..semantic_vector_caption.len() {
                        v.push(semantic_vector_caption[i] + semantic_vector_labels[i]);
                    }
                    for i in 0..v.len() {
                        v[i] = v[i] / 2.0;
                    }
                    v
                };
                let semantic_vector = SemanticVec::from_vec(semantic_vector);
                let mut image = Image::new(path.to_str().unwrap().to_string(), path.file_name().unwrap().to_str().unwrap().to_string());
                let conn = db.as_mut().unwrap().connection.as_mut().unwrap();
                image.set_semantic_vector(semantic_vector);
                match image.save(conn) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }).await.expect("Couldnt open dir");
}

