use std::{fs, io};
use std::sync::{Arc, Mutex};
use file_system::{dir_walker};
use crate::dir_walker::DirWalker;
use trie::trie::Trie;

fn main() -> io::Result<()> {
    if let Ok(mut it) = DirWalker::new("C:/Users/ikvict/") {
        // let trie = Arc::new(Mutex::new(Trie::new()));
        // let trie_clone = Arc::clone(&trie);
        // it.walk_apply(move |path| {
        //     let mut trie = trie_clone.lock().unwrap();
        //     trie.insert(path);
        // });
        // let trie = trie.lock().unwrap();
        // println!("{:?}", trie.search("image_2024-04-01_01-35-54.png"));

        // serialize(&trie)?;

        
        let deserialized: Trie = deserialize().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        println!("Deserialized: {:?}", deserialized.search("python.exe"));
        // let mut input: String;
        // loop {
        //     input = String::new();
        //     io::stdin().read_line(&mut input).unwrap();
        //     if input.trim() == "exit" {
        //         break;
        //     }
        //     println!("{:?}", deserialized.search(input.trim()));
        // }
    }
    Ok(())
}

fn serialize(trie: &Trie) -> io::Result<()> {
    let encoded: Vec<u8> = bincode::serialize(trie).unwrap();
    fs::write("trie.bin", encoded)
}

fn deserialize() -> bincode::Result<Trie> {
    let data = fs::read("trie.bin").unwrap();
    bincode::deserialize(&data)
}