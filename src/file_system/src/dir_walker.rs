use std::fs;
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;

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
}