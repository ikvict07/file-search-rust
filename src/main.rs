use std::{fs::File, io::{BufReader, BufWriter}, path::Path};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};
use dioxus::html::button;
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
use db::semantic_vector::SemanticVec;

// Important we rewrite import from dioxus
#[derive(Clone)]
enum ActiveWindow {
    StartWindow,
    FileSearch,
    FileIndex,
    ImageSearch,
    ImageIndex,
}

fn main() {
    let map = initialize_map();
    let app_props = App {
        map: map,
        trie: Arc::new(Mutex::new(SomeTrie::TrieBuilder(TrieBuilder::new()))),
        is_prefix_search_enabled: Arc::new(Mutex::new(false)),
        embeddings: initialize_embeddings(),
    };
    let app_props = Arc::new(Mutex::new(app_props));
    dioxus_desktop::launch_with_props(
        app,
        app_props,
        Config::default());
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
        }
    })
}


pub fn file_search(cx: Scope<Arc<Mutex<App>>>) -> Element {
    let input_value = use_state(&cx, || "".to_string());
    let found_files = use_state(&cx, || Vec::new());
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
                        if *(is_enabled.lock().unwrap()) { // Prefix search
                            if let SomeTrie::Trie(trie) = app.trie.lock().unwrap().deref() {
                                let found_names = trie.predictive_search(dir.clone().as_bytes());
                                let mut temp= Vec::new();
                                for (name) in found_names {
                                    let map_lock = app.map.lock().unwrap();
                                    for file in map_lock.get(&ArcStr(Arc::from(String::from_utf8(name.clone()).unwrap()))).unwrap() {
                                        temp.push(String::from_utf8(name.clone()).unwrap() + ": " + &*file.0.to_string());
                                    }
                                }
                                found_files.set(temp);
                            }
                            else {
                                panic!("Trie is not initialized");
                            }
                        } else { // No prefix search
                            let map_lock = app.map.lock().unwrap();
                            let mut temp= Vec::new();
                            if let Some(files) = map_lock.get(&ArcStr(Arc::from(dir.clone()))) {
                                for file in files {
                                    temp.push(dir.clone() + ": " + &*file.0.to_string());
                                }
                            }
                            found_files.set(temp);
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
                    index_directory(dir, &mut *app);;
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
    cx.render(rsx! {
        div {
            h1 { "Image Search Window" }
            button {

            }
        }
    })
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

fn handle_response_caption(response: Value) -> Result<(), Box<dyn Error>> {
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


pub async  fn index_images<'a> (dir: String, app: Arc<Mutex<App>>) {
    // let secret = String::from("client_secret.json");
    // let label_processor = ImageProcessor::new_label(dir.clone(), secret.clone());
    // let caption_processor = ImageProcessor::new_caption(dir, secret, 3);
    // let label_callback = Arc::new(Mutex::new(handle_response_label));
    // let caption_callback = Arc::new(Mutex::new(handle_response_caption));
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

    // let db = db::database::Database::new();
    // let vector = vec![2.0, 3.1, 4.3];
    // let semantic_vector = db::semantic_vector::SemanticVec::from_vec(vector);
    //
    //
    // if db.is_err() {
    //     return;
    // }
    // let db = db.unwrap();
    //
    // let mut image = Image::new(String::from("path4"), String::from("title2"));
    //
    // let conn = db.connection.as_ref();
    // if conn.is_none() {
    //     return;
    // }
    // println!("Db");
    // let conn = conn.unwrap();
    // image.set_semantic_vector(semantic_vector);
    // match image.save(conn) {
    //     Ok(_) => {}
    //     Err(e) => {
    //         println!("Error: {:?}", e);
    //     }
    // }
    //
    // let test = db.select_image_by_path("path4");
    //
    // match test {
    //     None => {
    //         db.close();
    //         return;
    //     }
    //     Some(_) => {}
    // }
    // let test = test.unwrap();
    // println!("Image: {:?}", test);
    //
    // db.close();
    // let walker = DirWalker::new(&dir).unwrap();
    // let mut embeddings = Embedding::new();
    // embeddings.get_embeddings(r"C:\Users\ikvict\RustroverProjects\file-search\glove.6B.300d.txt");
    // let embeddings = embeddings;
    // let db = db::database::Database::new().unwrap();
    // let db_arc = Arc::new(Mutex::new(db));
    // let db_for_closure = Arc::clone(&db_arc);
    // walker.send_requests_for_dir_apply(10, |resp, path| {
    //     let db = db_for_closure.lock().unwrap();
    //     match resp {
    //         Ok(response) => {
    //             let semantic_vector = SemanticVec::from_vec(embeddings.average_vector(&response.caption));
    //             let mut image = Image::new(path.to_str().unwrap().to_string(), path.file_name().unwrap().to_str().unwrap().to_string());
    //             let conn = db.connection.as_ref().unwrap();
    //             image.set_semantic_vector(semantic_vector);
    //             match image.save(conn) {
    //                 Ok(_) => {}
    //                 Err(e) => {
    //                     println!("Error: {:?}", e);
    //                 }
    //             }
    //         }
    //         Err(e) => {
    //             println!("Error: {:?}", e);
    //         }
    //     }
    // }).await.expect("Couldnt open dir");
    // db.close();
    let embeddings = app.lock().unwrap().embeddings.clone();
    let db = db::database::Database::new().unwrap();
    let embeddings = embeddings.lock().unwrap();

    let res = embeddings.average_vector(dir.as_str());

    let ids = db.select_all_images();
    let mut results = Vec::new();
    println!("{:?}", ids);
    let time = std::time::Instant::now();
    for id in ids {
        let mut statement = db.connection.as_ref().unwrap().prepare("SELECT value FROM semantic_vectors WHERE image_id = ?1").unwrap();
        let mut rows = statement.query(&[&id]).unwrap();
        let mut vec = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            let value: f32 = row.get(0).unwrap();
            vec.push(value);
        }
        results.push((id, Embedding::cosine_similarity(&res, &vec)));
    }

    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    for (id, value) in results {
        println!("Id: {}, Value: {}", id, value);
    }
    println!("Time: {:?}", time.elapsed());
    db.close();
}
