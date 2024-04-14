use std::mem::forget;
use rusqlite::{Connection, ErrorCode};
use crate::image::Image;
use crate::semantic_vector::{SemanticVec, SemanticVectorElement};
use fallible_iterator::FallibleIterator;

#[derive(Debug)]
pub struct Database {
    pub connection: Option<Connection>,
}

impl Database {
    pub fn new() -> Result<Database, rusqlite::Error> {
        let connection = Connection::open("database.db").expect("Connection::open");

        match connection
            .execute(
                "CREATE TABLE IF NOT EXISTS images (
                    id INTEGER PRIMARY KEY,
                    path TEXT NOT NULL UNIQUE,
                    title TEXT NOT NULL
                )",
                [],
            ) {
            Ok(_) => (),
            Err(e) => return Err(e),
        };
        match connection
            .execute(
                "CREATE INDEX IF NOT EXISTS index_images_on_path ON images (path)",
                [],
            ) {
            Ok(_) => (),
            Err(e) => return Err(e),
        };
        match connection
            .execute(
                "CREATE TABLE IF NOT EXISTS semantic_vectors (
                        id INTEGER PRIMARY KEY,
                        image_id INTEGER,
                        value REAL NOT NULL,
                        FOREIGN KEY(image_id) REFERENCES images(id)
                    )",
                [],
            ) {
            Ok(_) => (),
            Err(e) => return Err(e),
        };
        Ok(Database { connection: Some(connection) })
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        self.connection.take().unwrap().close().expect("Connection::close");
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
    fn save(&mut self, connection: &Connection) -> Result<u32, rusqlite::Error>;
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
        let mut rows = statement.query(&[&path]);

        if rows.is_err() {
            return None;
        }
        let mut rows = rows.unwrap();


        let row = rows.next();
        if row.is_err() {
            return None;
        }
        let row = row.unwrap();
        if row.is_none() {
            return None;
        }
        let row = row.unwrap();

        let image_id: Result<u32, rusqlite::Error> = row.get(0);
        if image_id.is_err() {
            return None;
        }
        let image_id = image_id.unwrap();

        let path: Result<String, rusqlite::Error> = row.get(1);
        if path.is_err() {
            return None;
        }
        let path = path.unwrap();

        let title: Result<String, rusqlite::Error> = row.get(2);
        if title.is_err() {
            return None;
        }
        let title = title.unwrap();


        let statement = self.connection.as_ref();

        if statement.is_none() {
            return None;
        }
        let mut statement = statement.unwrap()
            .prepare("SELECT id, image_id, value FROM semantic_vectors WHERE image_id = ?1")
            .expect("SELECT id, image_id, value FROM semantic_vectors WHERE image_id = ?1");

        let elements: rusqlite::Result<Vec<SemanticVectorElement>> = statement.query_map(&[&image_id], |row| {
            if row.get::<usize, u32>(0).is_err() {
                return Err(row.get::<usize, u32>(0).err().unwrap());
            }
            if row.get::<usize, String>(1).is_err() {
                return Err(row.get::<usize, String>(1).err().unwrap());
            }
            if row.get::<usize, String>(2).is_err() {
                return Err(row.get::<usize, String>(2).err().unwrap());
            }

            let id: u32 = row.get(0).unwrap();
            let image_id: u32 = row.get(1).unwrap();
            let value: f32 = row.get(2).unwrap();
            Ok(SemanticVectorElement {
                id,
                image_id,
                value,
            })
        }).unwrap().collect();

        if elements.is_err() {
            return None;
        }

        let semantic_vec = SemanticVec(elements.unwrap());


        Some(Image {
            id: image_id,
            path,
            title,
            semantic_vector: semantic_vec,
        })
    }

    pub fn exists_image_by_path(&self, path: &str) -> Result<bool, rusqlite::Error> {
        let mut statement = self.connection.as_ref().unwrap()
            .prepare("SELECT id FROM images WHERE path = ?1")
            .expect("SELECT id FROM images WHERE path = ?1");
        let mut rows = statement.query(&[&path]);
        if rows.is_err() {
            return Err(rows.err().unwrap());
        }
        let mut rows = rows.unwrap();

        Ok(rows.next().is_ok())
    }

    pub fn select_all_images(&self) -> Vec<u32> {
        let mut statement = self.connection.as_ref().unwrap()
            .prepare("SELECT id FROM images")
            .expect("pISUN");
        let mut rows = statement.query(());
        if rows.is_err() {
            return vec![];
        }
        let mut rows = rows.unwrap();


        let mut result = vec![];

        while let Ok(row) = rows.next() {
            if row.is_none() {
                break;
            }
            let row = row.unwrap();

            let image_id: Result<u32, rusqlite::Error> = row.get(0);
            if image_id.is_err() {
                break;
            }
            let image_id = image_id.unwrap();

            result.push(image_id)
        }
        result
    }
}