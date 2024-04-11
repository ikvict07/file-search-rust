use rusqlite::Connection;

#[derive(Debug, Clone)]
pub struct SemanticVectorElement {
    pub id : u32,
    pub image_id : u32,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct SemanticVec(pub Vec<SemanticVectorElement>);

impl crate::database::Save for Vec<SemanticVectorElement>
{
    fn save(&mut self, connection: &Connection) {
        for ( value) in self.iter_mut() {
            println!("saving {}", value.value);
            connection
                .execute(
                    "INSERT INTO semantic_vectors (image_id, value) VALUES (?1, ?2)",
                    (value.image_id, value.value),
                )
                .expect("INSERT INTO semantic_vectors");
            value.id = connection.last_insert_rowid() as u32;
        }
    }
}

impl crate::database::Save for SemanticVec
{
    fn save(&mut self, connection: &Connection) {
        println!("SemanticVec save");
        self.0.save(connection);
    }
}

impl SemanticVectorElement {
    pub fn new(image_id: u32, value: f64) -> SemanticVectorElement {
        SemanticVectorElement {
            id: 0,
            image_id,
            value,
        }
    }
}

impl SemanticVec {
    pub fn new() -> SemanticVec {
        SemanticVec(Vec::new())
    }

    pub fn push(&mut self, element: SemanticVectorElement) {
        self.0.push(element);
    }

    pub fn from_vec(vec: Vec<f64>) -> SemanticVec {
        let mut semantic_vector = SemanticVec::new();
        for value in vec {
            semantic_vector.push(SemanticVectorElement::new(0, value));
        }
        semantic_vector
    }
}
