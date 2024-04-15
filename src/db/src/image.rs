use rusqlite::Connection;
use crate::semantic_vector::{SemanticVec, SemanticVectorElement};

#[derive(Debug)]
pub struct Image {
    pub id: u32,
    pub path: String,
    pub title: String,
    pub semantic_vector: SemanticVec,
}

impl crate::database::Save for Image
{
    fn save(&mut self, connection: &mut Connection) -> Result<u32, rusqlite::Error> {
        match connection
            .execute(
                "INSERT INTO images (path, title) VALUES (?1, ?2)",
                &[&self.path, &self.title],
            ) {
            Ok(_) => {}
            Err(e) => {
                return Err(e);
            }
        }

        self.id = connection.last_insert_rowid() as u32;

        println!("Image save");
        self.set_semantic_vector(self.semantic_vector.clone());
        self.semantic_vector.save(connection);
        Ok(connection.last_insert_rowid() as u32)
    }
}

impl Image {
    pub fn set_semantic_vector(&mut self, mut semantic_vector: SemanticVec) {
        for element in semantic_vector.0.iter_mut() {
            element.image_id = self.id;
        }
        self.semantic_vector = semantic_vector;
    }

    pub fn new(path: String, title: String) -> Image {
        Image {
            id: 0,
            path,
            title,
            semantic_vector: SemanticVec::new(),
        }
    }
}


