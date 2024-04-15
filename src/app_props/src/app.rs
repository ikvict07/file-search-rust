use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use trie_rs::{Trie, TrieBuilder};
use db::database::Database;
use trie::arc_str::ArcStr;
use vectorization::Embedding;
pub enum SomeTrie {
    Trie(Trie<u8>),
    TrieBuilder(TrieBuilder<u8>),
}

pub struct App {
    pub map: Arc<Mutex<HashMap<ArcStr, HashSet<ArcStr>>>>,
    pub trie: Arc<Mutex<SomeTrie>>,
    pub is_prefix_search_enabled: Arc<Mutex<bool>>,
    pub embeddings: Arc<Mutex<Embedding>>,
    pub db: Arc<Mutex<Option<Database>>>
}

impl App {}
pub fn initialize_embeddings(app: Arc<Mutex<App>>) {
    let mut app_clone = app.clone();
    {
        let mut app = app.lock().unwrap();
        let mut embeddings = app.embeddings.lock().unwrap();
        embeddings.get_embeddings(r"C:\Users\ikvict\RustroverProjects\file-search\glove.6B.100d.txt");
        print!("Embeddings initialized\n");
    }
    {
        let mut app = app_clone.lock().unwrap();
        app.db = Arc::new(Mutex::new(Some(Database::new().unwrap())));
        println!("Database initialized");
    }
}
pub fn initialize_map() -> Arc<Mutex<HashMap<ArcStr, HashSet<ArcStr>>>> {
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

pub fn initialize_trie(map: &Arc<Mutex<HashMap<ArcStr, HashSet<ArcStr>>>>) -> Arc<Mutex<SomeTrie>> {
    let mut builder = TrieBuilder::new();
    let map = map.lock().unwrap();
    for key in map.keys() {
        builder.push(key.0.to_string());
    }
    let trie = builder.build();
    let trie = Arc::new(Mutex::new(SomeTrie::Trie(trie)));
    trie
}
pub fn build_trie(map: Arc<Mutex<HashMap<ArcStr, HashSet<ArcStr>>>>) -> SomeTrie {
    let mut builder = TrieBuilder::new();
    let map_lock = map.lock().unwrap();
    for (key, value) in map_lock.iter() {
        builder.push(key.0.to_string());
    }
    let trie_ = builder.build();
    SomeTrie::Trie(trie_)
}

pub fn enable_prefix_search(app: &Arc<Mutex<App>>) {
    let app = &mut *app.lock().unwrap();

    let is_enabled = &mut app.is_prefix_search_enabled;
    if !*is_enabled.lock().unwrap() {
        println!("Initializing prefix search");
        app.trie = initialize_trie(&app.map);
        app.is_prefix_search_enabled = Arc::from(Mutex::from(true));
        println!("Prefix search enabled");
        println!("Trie: {:?}", app.is_prefix_search_enabled);
    }
}
