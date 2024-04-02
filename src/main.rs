use file_system::{dir_walker};
use crate::dir_walker::DirWalker;
fn main() {
    if let Ok(mut it) = DirWalker::new("C:\\Users\\ikvict/Downloads") {
        it.walk_apply(|path| {
            println!("{}", path);
        });
    }
}
