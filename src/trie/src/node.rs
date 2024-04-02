use std::collections::HashMap;
use std::iter::Map;
use std::rc::Rc;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::arc_str::ArcStr;

#[derive(Serialize, Deserialize)]
pub struct Node {
    pub path: Option<Vec<ArcStr>>,
    pub children: HashMap<char, Box<Node>>,
    pub letter: char,
}
impl Node {
    pub fn new(letter: char) -> Self {
        Node {
            path: None,
            children: HashMap::new(),
            letter,
        }
    }
}