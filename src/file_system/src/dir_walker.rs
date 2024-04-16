use std::{fs, time};
use std::error::Error;
use std::io;
use std::io::ErrorKind;
use std::sync::{Arc, Mutex};
use std::thread;
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use futures::stream;
use futures::stream::StreamExt;
use serde_json::Value;
use img_azure::azure_api::*;
use db::database::Database;

pub struct DirWalker {
    dirs: Arc<Mutex<Vec<String>>>,
}

impl DirWalker {
    pub fn new(path: &str) -> Result<Self, io::Error> {
        let mut dirs = Vec::new();
        dirs.push(path.to_string());
        Ok(Self { dirs: Arc::new(Mutex::new(dirs)) })
    }

    pub fn walk(&self) {
        let dirs = Arc::clone(&self.dirs);
        let mut handles = vec![];

        for _ in 0..num_cpus::get() {
            let dirs = Arc::clone(&dirs);
            let handle = thread::spawn(move || {
                loop {
                    let dir = {
                        let mut dirs = dirs.lock().unwrap();
                        if let Some(dir) = dirs.pop() {
                            dir
                        } else {
                            break;
                        }
                    };

                    match fs::read_dir(&dir) {
                        Ok(entries) => {
                            for entry in entries {
                                match entry {
                                    Ok(entry) => {
                                        let path = entry.path();
                                        if path.is_dir() {
                                            let mut dirs = dirs.lock().unwrap();
                                            dirs.push(path.to_str().unwrap().to_string());
                                        } else if path.is_file() {
                                            println!("{}", path.to_str().unwrap());
                                        }
                                    }
                                    Err(e) => eprintln!("{}", e),
                                }
                            }
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    pub fn walk_apply(&self, f: impl Fn(&str) + Send + Sync + 'static) {
        let dirs = Arc::clone(&self.dirs);
        let mut handles = vec![];
        let f = Arc::new(f);
        for _ in 0..num_cpus::get() {
            let dirs = Arc::clone(&dirs);
            let f = Arc::clone(&f);
            let handle = thread::spawn(move || {
                loop {
                    let dir = {
                        let mut dirs = dirs.lock().unwrap();
                        if let Some(dir) = dirs.pop() {
                            dir
                        } else {
                            break;
                        }
                    };

                    match std::fs::read_dir(&dir) {
                        Ok(entries) => {
                            for entry in entries {
                                match entry {
                                    Ok(entry) => {
                                        let path = entry.path();
                                        if path.is_dir() {
                                            let mut dirs = dirs.lock().unwrap();
                                            dirs.push(path.to_str().unwrap().to_string());
                                        } else if path.is_file() {
                                            match path.to_str() {
                                                Some(path) => f(path),
                                                None => eprintln!("Invalid path"),
                                            }
                                        }
                                    }
                                    Err(e) => eprintln!("{}", e),
                                }
                            }
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }


    pub async fn send_requests_for_dir_apply(&self, db: Arc<Mutex<Option<Database>>>, requests_per_sec: u32, f: impl Fn(Result<AzureResponse, ErrorKind>, PathBuf) + Send + 'static) -> Result<(), Box<dyn Error>> {
        let dirs = self.dirs.clone();
        let dirs_arc = Arc::clone(&dirs);
        let current = time::Instant::now();
        let f = Arc::new(f);
        let db = db.clone();
        while let Some(dir) = {
            let mut dirs = dirs.lock().unwrap();
            dirs.pop()
        } {
            let dir_entries = tokio::fs::read_dir(dir).await?;

            let dir_entries_stream = stream::unfold(dir_entries, |mut dir_entries| async {
                match dir_entries.next_entry().await {
                    Ok(Some(entry)) => Some((entry, dir_entries)),
                    _ => None,
                }
            });

            let callback = f.clone();
            let limiter = Arc::new(RateLimiter::direct(
                Quota::per_second(NonZeroU32::new(requests_per_sec).unwrap()),
            ));

            let db = db.clone();
            let dirs_arc_clone = Arc::clone(&dirs_arc);

            dir_entries_stream.for_each_concurrent(None, move |entry| {
                let limiter = Arc::clone(&limiter);
                let callback = Arc::clone(&callback);
                let dirs_arc_clone = Arc::clone(&dirs_arc_clone);
                let db = Arc::clone(&db);
                let mut db = db.lock().unwrap();
                let path = entry.path();
                let mut flag = false;
                if (path.is_file()) {
                    if is_image(path.to_str().unwrap()) {
                        let db = db.as_mut().unwrap();
                        flag = db.exists_image_by_path(path.to_str().unwrap()).unwrap();
                    }
                }
                async move {
                    if flag {
                        return;
                    }
                    let path = entry.path();
                    if path.is_file() {
                        if !is_image(path.to_str().unwrap()) {
                            return;
                        }
                        let path_str = path.to_str().unwrap();


                        limiter.until_ready().await;

                        let mut request = AzureRequest::new("4d7bd39a70c249eebd19f5b8d62f5d7b", vec!["tags", "caption"]);
                        request.set_img(path_str).unwrap();
                        let response = request.send_request().await.unwrap();
                        let response_copy = response.json::<Value>().await.unwrap();
                        let response_struct: Result<AzureResponse, ErrorKind> = AzureResponse::try_from(response_copy.clone());

                        callback(response_struct, path);
                    } else if path.is_dir() {
                        let mut dirs = dirs_arc_clone.lock().unwrap();
                        dirs.push(path.to_str().unwrap().to_string());
                    }
                }
            }).await;
        }

        println!("Time elapsed: {:?}", current.elapsed());
        println!("Dirs {:?}", dirs.lock().unwrap());
        Ok(())
    }
}

fn is_image(path: &str) -> bool {
    if let Some(ext) = Path::new(path).extension() {
        return match ext.to_str() {
            Some("jpg") | Some("jpeg") | Some("png") => true,
            _ => false,
        };
    }
    false
}