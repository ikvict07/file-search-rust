use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufWriter;
use std::num::NonZeroU32;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use app_props::app::{App, SomeTrie};
use tokio;
use dioxus::prelude::*;
use std::sync::{Arc, Mutex};
use arc_str::arc_str::ArcStr;
use db::database::{Database, Save};
use db::semantic_vector::SemanticVec;
use img_azure::azure_api::AzureResponse;
use vectorization::Embedding;
use db::image::Image;
use file_system::dir_walker::DirWalker;
use governor::{Quota, RateLimiter};
use img_azure::get_response_by_path;


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
        None => { Ordering::Less }
        Some(res) => { res }
    }
    );
    println!("Time: {:?}", time.elapsed());
    results.iter().take(10).map(|(path, id, value)| (path.clone(), *id, *value)).collect()
}


fn prepare_semantic_vec(embeddings: Arc<Mutex<Embedding>>, response: &mut AzureResponse, label_vec: &mut Vec<String>) -> Vec<f32> {
    let semantic_vector_caption = embeddings.lock().unwrap().average_vector(&response.caption);
    let semantic_vector_labels = embeddings.lock().unwrap().average_vector(&label_vec.join(" "));
    let mut v = Vec::new();
    for i in 0..semantic_vector_caption.len() {
        v.push(semantic_vector_caption[i] + semantic_vector_labels[i]);
    }
    for i in 0..v.len() {
        v[i] = v[i] / 2.0;
    }
    v
}

async fn index_directory(dir: String, app: Arc<Mutex<App>>) {
    let start = std::time::Instant::now();
    let map_for_closure = {
        let app = app.lock().unwrap();
        app.map.clone()
    };
    let map_for_ser = {
        let app = app.lock().unwrap();
        app.map.clone()
    };
    if let Ok(it) = DirWalker::new(&dir) {
        it.walk(move |path| {
            let path = path.to_owned().replace("\\", "/");
            let map_for_closure = map_for_closure.clone();
            async move {
                let path_string = Path::new(&path);
                let filename = path_string.file_name().unwrap().to_str().unwrap().to_string().replace("\\", "/");
                let filename_arc: Arc<str> = Arc::from(filename.clone());
                let path_arc: Arc<str> = Arc::from(path);

                let mut map = map_for_closure.lock().unwrap();
                if !map.contains_key(&ArcStr(filename_arc.clone())) {
                    let mut v = HashSet::new();
                    v.insert(ArcStr(path_arc.clone()));
                    map.insert(ArcStr(filename_arc.clone()), v);
                } else {
                    map.get_mut(&ArcStr(filename_arc.clone())).unwrap().insert(ArcStr(path_arc.clone()));
                }
            }
        }).await;

        let file = File::create("map.bin").expect("Unable to create file");
        let writer = BufWriter::new(file);


        let map = map_for_ser.lock().unwrap();
        if bincode::serialize_into(writer, &*map).is_err() {
            println!("Error serializing map");
        }
        println!("Directory indexed");
        println!("Time: {:?}", start.elapsed());
    }
}

pub fn file_search(cx: Scope<Arc<Mutex<App>>>) -> Element {
    let input_value = use_state(&cx, || "".to_string());
    let found_files: &UseState<Vec<String>> = use_state(&cx, || Vec::new());
    let _results_state: &UseState<Vec<(String, u32, f32)>> = use_state(&cx, || Vec::new()); //uselles

    let files: &Vec<String> = found_files.get();

    cx.render(rsx! {
        div {
            class: "container centered",

            div {
                class: "row",
                div {
                    class: "col-12 d-flex justify-content-center align-items-center",
                    p { "File Search" },
                }
            }
            div {
                class: "row",
                div {
                    class: "col-12 d-flex justify-content-center align-items-center",
                    div {
                        style: "display: flex; justify-content: center; align-items: center;",
                        input {
                            placeholder: "Filename or prefix",
                            value: "{input_value}",
                            oninput: move |event| {
                                let input = &event.value;
                                input_value.set(input.to_string());
                            }
                        }
                    }
                }
            }
            div {
                class: "row",
                div {
                    class: "col-12 d-flex justify-content-center align-items-center",
                    div {
                        class: "menu-btn1",
                        style: "width: 100px",
                        onclick: move |_| {
                            let results= on_click_file_search(input_value.get().clone(), cx.props);
                            found_files.set(results);
                        },
                        "Search"
                    }
                }
            }

        }
        div {
            class: "file-container",
            div {
                class: "row",
                for file in files {
                    div {
                        class: "col-12",
                        p {
                            file.clone()
                        }
                    }
                }
            }
        }

    })
}

pub fn image_search(cx: Scope<Arc<Mutex<App>>>) -> Element {
    let input_value = use_state(&cx, || "".to_string());
    let _found_files: &UseState<Vec<String>> = use_state(&cx, || Vec::new()); //uselles here only to satisfy hook order
    let results_state: &UseState<Vec<(String, u32, f32)>> = use_state(&cx, || Vec::new());
    let app = cx.props.clone();
    let is_enabled = {
        let app = app.lock().unwrap();
        app.is_image_search_enabled.load(std::sync::atomic::Ordering::Relaxed)
    };

    if !is_enabled {
        cx.render(rsx! {
            div {
                class: "container centered",

                div {
                    style: "flex-wrap: wrap;",
                    div {
                        class: "row",
                        div {
                            class: "col-md-12",
                            div {
                                style: "display: flex; justify-content: center; align-items: center;",
                                p {
                                    style: "display: flex; justify-content: center; align-items: center;",
                                    "Image Search"
                                }
                            }
                        }
                    }
                    div {
                        class: "row",
                        div {
                            class: "col-md-12",
                            div {
                                style: "display: flex; justify-content: center; align-items: center;",
                                p {
                                    style: "display: flex; justify-content: center; align-items: center;",
                                    "Enable this option on Start Window"
                                }
                            }
                        }
                    }
                }
            }
        })
    } else {
        cx.render(rsx! {
            div {
                class: "container centered",

                div {
                    class: "row",
                    div {
                        class: "col-12 d-flex justify-content-center align-items-center",
                        p { "Image Search" }
                    }
                    div {
                        class: "col-12",
                        div {
                            style: "display: flex; justify-content: center; align-items: center;",
                            input {
                                placeholder: "Describe photo to search",
                                value: "{input_value}",
                                oninput: move |event| {
                                    let input = &event.value;
                                    input_value.set(input.to_string());
                                }
                            }
                        }
                    }
                    div {
                        class: "col-12",
                        div {
                            style: "display: flex; justify-content: center; align-items: center;",
                            div {
                                class: "menu-btn1",
                                onclick: move |_| {
                                    let r = on_click_image_search(input_value.get().clone(), cx.props.clone());
                                    results_state.set(r.clone());
                                },
                                "Find Photo"
                            }
                        }
                    }
                }
            }
            div {
                class: "file-container",
                div {
                    class: "row",
                    for (path, id, value) in results_state.get().iter() {
                        div {
                            class: "col-12",
                            div {
                                class: "file-p",
                                img {
                                    src: &**path,
                                    width: "200",
                                    height: "200"
                                }
                                div {
                                    format!("Path: {}, Id: {}, Value: {}", path, id, value)
                                }
                            }
                        }
                    }
                }
            }
            // div {
            //     h1 { "Image Search Window" }
            //     input {
            //         value: "{input_value}",
            //         oninput: move |event| {
            //             let input = &event.value;
            //             input_value.set(input.to_string());
            //         }
            //     }
            //     button {
            //         onclick: move |_| {
            //             let r = on_click_image_search(input_value.get().clone(), cx.props.clone());
            //             results_state.set(r.clone());
            //         },
            //         "Поиск изображений"
            //     }
            //     div {
            //         for (path, id, value) in results_state.get().iter() {
            //             img {
            //                 src: &**path,
            //                 width: "100",
            //                 height: "100"
            //             }
            //             div {
            //                 format!("Path: {}, Id: {}, Value: {}", path, id, value)
            //             }
            //         }
            //     }
            // }
        })
    }
}

pub fn file_index(cx: Scope<Arc<Mutex<App>>>) -> Element {
    let input_value = use_state(&cx, || "".to_string());
    let app = cx.props.clone();
    cx.render(rsx! {
        div {
            class: "container centered",

            div {
                class: "row",
                div {
                    class: "col-12",
                    div {
                        style: "display: flex; justify-content: center; align-items: center;",
                        p { "Index" }
                    }
                }
                div {
                    class: "col-md-12",
                    div {
                        style: "display: flex; justify-content: center; align-items: center;",
                        input {
                            placeholder: "/path/to/directory",
                            value: "{input_value}",
                            oninput: move |event| {
                                let input = &event.value;
                                input_value.set(input.to_string());
                            }
                        }
                    }
                }
                div {
                    class: "col-md-12",
                    div {
                        style: "display: flex; justify-content: center; align-items: center;",
                        div {
                            class: "menu-btn1",
                            style: "width: auto",
                            onclick: move |_| {
                                let dir = input_value.get().clone().replace("\\", "/");
                                let app_clone = app.clone();
                                tokio::spawn(async move {
                                    index_directory(dir, app_clone).await;
                                });
                            },
                            "Index Directory"
                        }
                    }
                }

            }
        }
        // div {
        //     class: "hero_area",
        //     style: "display: flex; justify-content: center; align-items: center;",
        //     section {
        //         class: "experience_section layout_padding-top layout_padding2-bottom",
        //         div {
        //             class: "container",
        //             div {
        //                 class: "row",
        //                 div {
        //                     div {
        //                         class: "col-md-12",
        //                         div {
        //                             style: "display: flex; justify-content: center; align-items: center;",
        //                                 p { "Index" }
        //                         }
        //                     }
        //                     div {
        //                         class: "col-md-12",
        //                         div {
        //                             style: "display: flex; justify-content: center; align-items: center;",
        //                             input {
        //                                 placeholder: "/path/to/directory",
        //                                 value: "{input_value}",
        //                                 oninput: move |event| {
        //                                     let input = &event.value;
        //                                     input_value.set(input.to_string());
        //                                 }
        //                             }
        //                         }
        //                     }
        //                     div {
        //                         class: "col-md-12",
        //                         div {
        //                             class: "detail-box",
        //                             style: "display: flex; justify-content: center; align-items: center;",
        //
        //                             div {
        //                                 class: "btn-box",
        //                                 a {
        //                                     class: "btn-3",
        //                                     onclick: move |_| {
        //                                         let dir = input_value.get().clone().replace("\\", "/");
        //                                         let app_clone = app.clone();
        //                                         tokio::spawn(async move {
        //                                             index_directory(dir, app_clone).await;
        //                                         });
        //                                     },
        //                                     "Index Directory"
        //                                 }
        //                             }
        //                         }
        //                     }
        //                 }
        //
        //             }
        //         }
        //     }
        // }
        // div {
        //     h1 { "Окно индексации файлов" }
        //     input {
        //         value: "{input_value}",
        //         oninput: move |event| {
        //             let input = &event.value;
        //             input_value.set(input.to_string());
        //         }
        //     }
        //     button {
        //         onclick: move |_| {
        //             let dir = input_value.get().clone();
        //             let app_clone = app.clone();
        //             tokio::spawn(async move {
        //                 index_directory(dir, app_clone).await;
        //             });
        //         },
        //         "Индексировать директорию"
        //     }
        // }
    })
}

pub fn on_click_file_search(filename: String, app: &Arc<Mutex<App>>) -> Vec<String> {
    let mut app = app.lock().unwrap();
    let is_enabled = &mut app.is_prefix_search_enabled;
    let mut result = Vec::new();
    if is_enabled.load(core::sync::atomic::Ordering::Relaxed) { // Prefix search
        if let SomeTrie::Trie(trie) = app.trie.lock().unwrap().deref() {
            let found_names = trie.predictive_search(filename.clone().as_bytes());
            for name in found_names {
                let map_lock = app.map.lock().unwrap();
                for file in map_lock.get(&ArcStr(Arc::from(String::from_utf8(name.clone()).unwrap()))).unwrap() {
                    result.push(String::from_utf8(name.clone()).unwrap() + ": " + &*file.0.to_string());
                }
            }
        } else {
            panic!("Trie is not initialized");
        }
    } else { // No prefix search
        let map_lock = app.map.lock().unwrap();
        if let Some(files) = map_lock.get(&ArcStr(Arc::from(filename.clone()))) {
            for file in files {
                result.push(filename.clone() + ": " + &*file.0.to_string());
            }
        }
    }
    result
}

pub fn on_click_image_search(prompt: String, app: Arc<Mutex<App>>) -> Vec<(String, u32, f32)> {
    let mut r = Vec::new();
    let (tx, rx) = std::sync::mpsc::channel();
    tokio::spawn(async move {
        let results = search_images(prompt, app).await;
        tx.send(results).unwrap();
    });
    let results = rx.recv().unwrap();
    for (path, id, value) in results.iter() {
        let is_windows_os = cfg!(target_os = "windows");
        let src;
        if is_windows_os {
            src = format!("/{}", path);
        } else {
            src = format!("{}", path);
        }
        r.push((src.clone(), *id, *value));
    }
    r
}

pub fn image_index(cx: Scope<Arc<Mutex<App>>>) -> Element {
    let input_value = use_state(&cx, || "".to_string());
    let app = cx.props.clone();
    let is_enabled = {
        let app = app.lock().unwrap();
        app.is_image_search_enabled.load(std::sync::atomic::Ordering::Relaxed)
    };
    if !is_enabled {
        cx.render(rsx! {
            div {
                class: "container centered",

                div {
                    style: "flex-wrap: wrap;",
                    div {
                        class: "row",
                        div {
                            class: "col-md-12",
                            div {
                                style: "display: flex; justify-content: center; align-items: center;",
                                p {
                                    style: "display: flex; justify-content: center; align-items: center;",
                                    "Image Indexing"
                                }
                            }
                        }
                    }
                    div {
                        class: "row",
                        div {
                            class: "col-md-12",
                            div {
                                style: "display: flex; justify-content: center; align-items: center;",
                                p {
                                    style: "display: flex; justify-content: center; align-items: center;",
                                    "Enable this option on Start Window"
                                }
                            }
                        }
                    }
                }
            }
        })
    } else {
        cx.render(rsx! {
            div {
                class: "container centered",
                div {
                    class: "row",
                    div {
                        class: "col-12",
                        div {
                            style: "display: flex; justify-content: center; align-items: center;",
                                p { "Image Indexing" }
                        }
                        div {
                            class: "col-12",
                            div {
                                style: "display: flex; justify-content: center; align-items: center;",
                                input {
                                    placeholder: "/path/to/directory",
                                    value: "{input_value}",
                                    oninput: move |event| {
                                        let input = &event.value;
                                        input_value.set(input.to_string());
                                    }
                                }
                            }
                        }
                        div {
                            class: "col-12",
                            div {
                                style: "display: flex; justify-content: center; align-items: center;",
                                div {
                                    class: "menu-btn1",
                                    onclick: move |_| {
                                        let dir = input_value.get().clone().replace("\\", "/");
                                        let app_clone = app.clone();
                                        tokio::spawn(async move {
                                            index_images(dir, app_clone).await;
                                        });
                                    },
                                    "Index Photos"
                                }
                            }
                        }
                    }
                }
            }
        })
    }
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
    let db_for_send = db.clone();
    let limiter = Arc::new(RateLimiter::direct(
        Quota::per_second(NonZeroU32::new(10).unwrap()),
    ));

    let db_for_send_clone = Arc::clone(&db_for_send);
    let embeddings_clone = Arc::clone(&embeddings);
    let limiter_clone = Arc::clone(&limiter);

    walker.walk(move |path| {
        let db_for_closure = Arc::clone(&db_for_send_clone);
        let embeddings = embeddings_clone.clone();
        let limiter = limiter_clone.clone();
        let path = path.to_owned();
        async move {
            let db_for_closure = Arc::clone(&db_for_closure);
            let path_buf = PathBuf::from(path.clone());
            let embeddings = embeddings.clone();
            let limiter = limiter.clone();
            let db_clone = db_for_closure.clone();
            if !path_buf.is_file() {
                return;
            }
            if !DirWalker::is_image(&path) {
                return;
            }

            if should_skip_image(db_clone, &path_buf) {
                return;
            }

            let limiter = Arc::clone(&limiter);
            let path_clone = path.to_string();


            limiter.until_ready().await;
            println!("indexing{}", path_clone);
            let results = get_response_by_path(&path_clone).await;


            let resp = results;
            if resp.is_err() {
                println!("Error: {:?}", resp.err().unwrap());
                return;
            }
            let mut response = resp.unwrap();
            let mut label_vec = Vec::new();
            response.labels.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
            for label in response.labels.iter().take(10) {
                label_vec.push(label.name.clone());
            }

            let semantic_vector = prepare_semantic_vec(embeddings.clone(), &mut response, &mut label_vec);
            let semantic_vector = SemanticVec::from_vec(semantic_vector);

            let mut image = Image::new(path.to_string(), path_buf.file_name().unwrap().to_str().unwrap().to_string());
            image.set_semantic_vector(semantic_vector);

            let mut db = db_for_closure.lock().unwrap();
            let conn = db.as_mut().unwrap().connection.as_mut().unwrap();
            match image.save(conn) {
                Ok(_) => {}
                Err(e) => {
                    println!("Error: {:?}", e);
                }
            }
        }
    }).await;
    println!("Indexing finished");
}

pub fn should_skip_image(db: Arc<Mutex<Option<Database>>>, path: &PathBuf) -> bool {
    let mut flag = false;
    if path.is_file() {
        if DirWalker::is_image(path.to_str().unwrap()) {
            let res = im::image_dimensions(path);
            if res.is_err() {
                flag = true;
                println!("skip0 {}", path.to_str().unwrap());
                return flag;
            }
            let (width, height) = res.unwrap();
            if width < 50 || height < 50 || width > 16000 || height > 16000 {
                flag = true;
                println!("skip1 {}", path.to_str().unwrap());
                return flag;
            } else {
                let mut db = db.lock().unwrap();
                let db = db.as_mut().unwrap();
                flag = db.exists_image_by_path(path.to_str().unwrap()).unwrap();
                if flag {
                    println!("skip2 {}", path.to_str().unwrap());
                }
                return flag;
            }
        }
    }
    flag
}