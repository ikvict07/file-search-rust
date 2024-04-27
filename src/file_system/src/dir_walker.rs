use std::{fs, time};
use std::future::Future;
use std::io;
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use futures::stream;
use futures::stream::StreamExt;
use std::sync::Condvar;


pub struct DirWalker {
    dirs: Arc<Mutex<Vec<String>>>,
}

impl DirWalker {
    pub fn new(path: &str) -> Result<Self, io::Error> {
        let mut dirs = Vec::new();
        dirs.push(path.to_string());
        Ok(Self { dirs: Arc::new(Mutex::new(dirs)) })
    }

    pub async fn walk<Fut>(&self, f: impl Fn(&str) -> Fut + Send + Sync + 'static)
    where Fut: Future<Output=()> + Send + 'static
    {
        let start = time::Instant::now();
        let mut handles = vec![];
        let f = Arc::new(f);

        let num_threads = num_cpus::get() - 1;
        let sleeping_threads = Arc::new(AtomicUsize::new(0));
        let pair = Arc::new((self.dirs.clone(), Condvar::new()));

        for _ in 0..num_threads {
            let pair2 = pair.clone();
            let sleeping_threads = sleeping_threads.clone();
            let f = f.clone();
            let handle = tokio::spawn(async move {
                loop {
                    let (lock, cvar) = &*pair2;
                    let dir = {
                        let mut dirs = lock.lock().unwrap();
                        while dirs.is_empty() {
                            sleeping_threads.fetch_add(1, Ordering::SeqCst);
                            if sleeping_threads.load(Ordering::SeqCst) == num_threads {
                                println!("All sleep");
                                cvar.notify_all();
                                return;
                            }
                            dirs = cvar.wait(dirs).unwrap();
                            if sleeping_threads.load(Ordering::SeqCst) == num_threads {
                                return;
                            }
                            sleeping_threads.fetch_sub(1, Ordering::SeqCst);
                        }
                        dirs.pop()
                    };
                    if let Some(dir) = dir {
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

                        dir_entries_stream.for_each_concurrent(None, move |entry| {
                            let callback = callback.clone();
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
                                    callback(path_str).await;
                                } else if path.is_dir() {
                                    Self::add_dir(lock.clone(), &path, cvar);
                                }
                            }
                        }).await;
                    }
                }
            });
            handles.push(handle);
        }


        for handle in handles {
            handle.await.unwrap();
        }
        println!("Elapsed time: {:?}", start.elapsed());
    }

    fn add_dir(dirs_arc_clone: Arc<Mutex<Vec<String>>>, path: &PathBuf, cvar: &Condvar) {
        if Self::is_symlink(&path) {
            return;
        };
        let mut dirs = dirs_arc_clone.lock().unwrap();
        dirs.push(path.to_str().unwrap().to_string());
        cvar.notify_one();
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
