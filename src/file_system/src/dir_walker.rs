use std::{fs, time};
use std::error::Error;
use std::future::Future;
use std::io;
use std::io::ErrorKind;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use futures::stream;
use futures::stream::StreamExt;
use serde_json::Value;
use img_azure::azure_api::*;
use db::database::Database;
use db::image;
use im::GenericImageView;

pub struct DirWalker {
    dirs: Arc<Mutex<Vec<String>>>,
}

impl DirWalker {
    pub fn new(path: &str) -> Result<Self, io::Error> {
        let mut dirs = Vec::new();
        dirs.push(path.to_string());
        Ok(Self { dirs: Arc::new(Mutex::new(dirs)) })
    }

    pub fn walk_apply(&self, f: impl Fn(&str) + Send + Sync + 'static) {
        let mut handles = vec![];
        let f = Arc::new(f);
        for _ in 0..num_cpus::get() {
            let dirs = self.dirs.clone();
            let f = f.clone();
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
                    let result = fs::read_dir(&dir);
                    if result.is_err() {
                        eprintln!("{}", result.err().unwrap());
                        continue;
                    }
                    let entries = result.unwrap();
                    for entry in entries {
                        if entry.is_err() {
                            eprintln!("{}", entry.err().unwrap());
                            continue;
                        }
                        let entry = entry.unwrap();

                        let path = entry.path();
                        if Self::is_symlink(&path) {
                            continue;
                        };
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
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }


    pub async fn send_requests_for_dir_apply(&self, db: Arc<Mutex<Option<Database>>>, requests_per_sec: u32, f: impl Fn(Result<AzureResponse, ErrorKind>, PathBuf) + Send + Sync + 'static) -> Result<(), Box<dyn Error>> {
        let current = time::Instant::now();
        let mut handles = vec![];
        let f = Arc::new(f);
        for _ in 0..num_cpus::get() {
            let dirs = self.dirs.clone();
            let f = f.clone();
            let db = db.clone();
            let handle = tokio::spawn(async move {
                'inner: while let Some(dir) = {
                    let dirs = dirs.lock();
                    if dirs.is_err() {
                        break 'inner;
                    }
                    let mut dirs = dirs.unwrap();
                    dirs.pop()
                } {
                    let dir_entries = tokio::fs::read_dir(dir).await;
                    if dir_entries.is_err() {
                        continue;
                    }
                    let dir_entries = dir_entries.unwrap();

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
                    let dirs_arc_clone = Arc::clone(&dirs);

                    dir_entries_stream.for_each_concurrent(None, move |entry| {
                        let limiter = limiter.clone();
                        let callback = callback.clone();
                        let dirs_arc_clone = Arc::clone(&dirs_arc_clone);
                        let db = db.clone();
                        let path = entry.path();
                        let flag = Self::should_skip(db, &path);
                        async move {
                            if flag {
                                return;
                            }
                            let path = entry.path();
                            if path.is_file() {
                                let path_str = path.to_str().unwrap();
                                if !Self::is_image(path.to_str().unwrap()) {
                                    return;
                                };
                                if Self::is_symlink(&path) {
                                    return;
                                };
                                limiter.until_ready().await;
                                let response_struct = Self::get_response(path_str).await;
                                callback(response_struct, path);
                            } else if path.is_dir() {
                                Self::add_dir(dirs_arc_clone, &path);
                            }
                        }
                    }).await;
                }
            });
            handles.push(handle);
        }
        for handle in handles {
            handle.await?;
        }

        println!("Time elapsed: {:?}", current.elapsed());
        println!("Dirs {:?}", self.dirs.lock().unwrap());
        Ok(())
    }

    async fn get_response(path_str: &str) -> Result<AzureResponse, ErrorKind> {
        let mut request = AzureRequest::new("4d7bd39a70c249eebd19f5b8d62f5d7b", vec!["tags", "caption"]);
        request.set_img(path_str).unwrap();
        let response = request.send_request().await.unwrap();
        let response_copy = response.json::<Value>().await.unwrap();
        let response_struct: Result<AzureResponse, ErrorKind> = AzureResponse::try_from(response_copy.clone());
        response_struct
    }

    fn add_dir(dirs_arc_clone: Arc<Mutex<Vec<String>>>, path: &PathBuf) {
        if Self::is_symlink(&path) {
            return;
        };
        let mut dirs = dirs_arc_clone.lock().unwrap();
        dirs.push(path.to_str().unwrap().to_string());
    }

    fn is_symlink(path: &PathBuf) -> bool {
        if path.is_symlink() {
            return true;
        }
        let metadata = fs::symlink_metadata(&path);
        if metadata.is_err() {
            return true;
        }
        let metadata = metadata.unwrap();
        if metadata.file_type().is_symlink() {
            return true;
        }
        false
    }
    fn should_skip(mut db: Arc<Mutex<Option<Database>>>, path: &PathBuf) -> bool {
        let mut db = db.lock().unwrap();
        let mut flag = false;
        if (path.is_file()) {
            if Self::is_image(path.to_str().unwrap()) {
                let img = im::open(path.to_str().unwrap());
                if img.is_err() {
                    println!("skip{}", path.to_str().unwrap());
                    flag = true;
                } else {
                    let img = img.unwrap();
                    let (width, height) = img.dimensions();
                    if width < 50 || height < 50 || width > 16000 || height > 16000 {
                        flag = true;
                    } else {
                        let db = db.as_mut().unwrap();
                        flag = db.exists_image_by_path(path.to_str().unwrap()).unwrap();
                    }
                }
            }
        }
        flag
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
}
