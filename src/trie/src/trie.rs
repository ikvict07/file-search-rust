use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::node::Node;
use crate::arc_str::ArcStr;

#[derive(Serialize, Deserialize)]
pub struct Trie {
    root: Node,
}

impl Trie {
    pub fn new() -> Self {
        Trie {
            root: Node::new(' '),
        }
    }

    pub fn insert(&mut self, path: &str) {
        let path_obj = Path::new(path);
        let mut current = &mut self.root;

        if let Some(file_name) = path_obj.file_name() {
            for letter in file_name.to_string_lossy().chars() {
                if !current.children.contains_key(&letter) {
                    let new_node = Node::new(letter);
                    current.children.insert(letter, Box::from(new_node));
                }
                current = current.children.get_mut(&letter).unwrap();
            }
        }

        match &mut current.path {
            None => {
                let mut paths = Vec::new();
                paths.push(ArcStr(Arc::from(path)));
                current.path = Some(paths);
            }
            Some(paths) => {
                paths.push(ArcStr(Arc::from(path)));
            }
        }
    }

    pub fn search(&self, filename: &str) -> Option<Vec<ArcStr>> {
        let mut current = &self.root;
        for letter in filename.chars() {
            if let Some(child) = current.children.get(&letter) {
                current = child;
            } else {
                return None;
            }
        }
        current.path.clone()
    }
}