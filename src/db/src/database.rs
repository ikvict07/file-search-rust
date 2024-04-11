use std::mem::forget;
use rusqlite::Connection;
use crate::image::Image;
use crate::semantic_vector::{SemanticVec, SemanticVectorElement};
use fallible_iterator::FallibleIterator;
pub struct Database {
    pub connection: Option<Connection>,
}

impl Database {
    pub fn new() -> Database {
        let connection = Connection::open("database.db").expect("Connection::open");
        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS images (
                    id INTEGER PRIMARY KEY,
                    path TEXT NOT NULL UNIQUE,
                    title TEXT NOT NULL
                )",
                [],
            )
            .expect("CREATE TABLE images");
        connection
            .execute(
                "CREATE INDEX IF NOT EXISTS index_images_on_path ON images (path)",
                [],
            )
            .expect("CREATE INDEX index_images_on_path");
        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS semantic_vectors (
                    id INTEGER PRIMARY KEY,
                    image_id INTEGER,
                    value REAL NOT NULL,
                    FOREIGN KEY(image_id) REFERENCES images(id)
                )",
                [],
            )
            .expect("CREATE TABLE semantic_vectors");
        Database { connection : Some(connection) }
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        panic!("Should be closed explicitly with close() method");
    }
}

impl Database {
    pub fn close(mut self) {
        self.connection.take().unwrap().close().expect("Connection::close");
        forget(self);
    }
}

pub trait Save {
    fn save(&mut self, connection: &Connection);
}

impl Database {
    pub fn save<T: Save>(&mut self, item: &mut T) {
        item.save(self.connection.as_ref().unwrap());
    }

    pub fn save_all<T: Save>(&mut self, items: &mut Vec<T>) {
        for item in items.iter_mut() {
            self.save(item);
        }
    }

    pub fn select_image_by_path(&self, path: &str) -> Option<Image> {
        let mut statement = self.connection.as_ref().unwrap()
            .prepare("SELECT id, path, title FROM images WHERE path = ?1")
            .expect("SELECT id, path, title FROM images WHERE path = ?1");
        let mut rows = statement.query(&[&path]).expect("query");
        let row = rows.next().expect("next").expect("row");

        let image_id: u32 = row.get(0).unwrap();


        let mut statement = self.connection.as_ref().unwrap()
            .prepare("SELECT id, image_id, value FROM semantic_vectors WHERE image_id = ?1")
            .expect("SELECT id, image_id, value FROM semantic_vectors WHERE image_id = ?1");
        let elements: rusqlite::Result<Vec<SemanticVectorElement>> = statement.query_map(&[&image_id], |row| {
            Ok(SemanticVectorElement {
                id: row.get(0)?,
                image_id: row.get(1)?,
                value: row.get(2)?,
            })
        }).unwrap().collect();

        let semantic_vec = SemanticVec(elements.unwrap());


        Some(Image {
            id: image_id,
            path: row.get(1).unwrap(),
            title: row.get(2).unwrap(),
            semantic_vector: semantic_vec,
        })
    }

    pub fn exists_image_by_path(&self, path: &str) -> bool {
        let mut statement = self.connection.as_ref().unwrap()
            .prepare("SELECT id FROM images WHERE path = ?1")
            .expect("SELECT id FROM images WHERE path = ?1");
        let mut rows = statement.query(&[&path]).expect("query");
        rows.next().is_some()
    }
}