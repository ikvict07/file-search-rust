use std::{fs, time};
use std::io;
use std::io::ErrorKind;
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use futures::stream;
use futures::stream::StreamExt;
use serde_json::Value;



pub struct DirWalker {
    dirs: Arc<Mutex<Vec<String>>>,
}

impl DirWalker {
    pub fn new(path: &str) -> Result<Self, io::Error> {
        let mut dirs = Vec::new();
        dirs.push(path.to_string());
        Ok(Self { dirs: Arc::new(Mutex::new(dirs)) })
    }

    pub async fn walk(&self, f: impl Fn(&str) + Send + Sync + 'static) {
        let mut handles = vec![];
        let f = Arc::new(f);
        for _ in 0..num_cpus::get() {
            let dirs = self.dirs.clone();
            let f = f.clone();
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
                    let dirs_arc_clone = Arc::clone(&dirs);


                    dir_entries_stream.for_each_concurrent(None, move |entry| {
                        let callback = callback.clone();
                        let dirs_arc_clone = Arc::clone(&dirs_arc_clone);
                        async move {
                            let path = entry.path();
                            if Self::is_symlink(&path) {
                                return;
                            };
                            if path.is_file() {
                                let path_str = path.to_str().unwrap();

                                if Self::is_symlink(&path) {
                                    return;
                                };
                                callback(path_str);
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
            handle.await.unwrap();
        }
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
    pub fn is_image(path: &str) -> bool {
        if let Some(ext) = Path::new(path).extension() {
            return match ext.to_str() {
                Some("jpg") | Some("jpeg") | Some("png") => true,
                _ => false,
            };
        }
        false
    }
}
