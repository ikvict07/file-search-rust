use std::error::Error;
use rust2vec::prelude::*;
use std::fs::File;
use std::io::BufReader;
use ndarray::Array1;
use std::time::Instant;
use memmap::Mmap;


pub struct Embedding {
    embeddings: Embeddings<SimpleVocab, NdArray>,
}

impl Embedding {
    pub fn new() -> Self {
        Embedding {
            embeddings: Embeddings::new(None, SimpleVocab::new(vec!["<UNK>".to_owned()]), NdArray(Default::default()), )
        }
    }

    pub fn get_embeddings(&mut self, path: &str) {
        let start = Instant::now();
        println!("Start loading embeddings");
        let file = File::open(path).unwrap();
        let mmap = unsafe { Mmap::map(&file).unwrap() };

        let mut reader = BufReader::new(&*mmap);
        self.embeddings = Embeddings::read_text(&mut reader, true).unwrap();

        println!("embeddings are loaded!!!\nTime: {:?}", start.elapsed());
    }

    fn prepare_text(text: &str) -> Vec<String> {
        let mut tokens: String = String::new();
        for i in text.chars() {
            if i.is_alphabetic() || i == ' ' {
                tokens.push(i);
            }
        }

        tokens = tokens.to_lowercase();
        tokens.split_whitespace().map(|s| s.to_string()).collect()
    }
    pub fn average_vector(&mut self, sentence: &str) -> Vec<f32> {
        println!("Sentence: {:?}", sentence);
        let words: Vec<String> = Self::prepare_text(sentence);
        println!("After split: {:?}", words);
        let mut vector = vec![0.0; self.embeddings.dims()];
        let mut count = 0;

        for word in words {
            if let Some(embedding) = self.embeddings.embedding(word.as_str()) {
                for (i, value) in embedding.as_view().iter().enumerate() {
                    vector[i] += *value;
                }
                count += 1;
            }
        }

        if count > 0 {
            for value in &mut vector {
                *value /= count as f32;
            }
        }

        vector
    }

    pub fn cosine_similarity(vector1: &[f32], vector2: &[f32]) -> f32 {
        let start = Instant::now();

        let dot_product: f32 = vector1.iter().zip(vector2).map(|(a, b)| a * b).sum();
        let magnitude1: f32 = vector1.iter().map(|a| a.powi(2)).sum::<f32>().sqrt();
        let magnitude2: f32 = vector2.iter().map(|a| a.powi(2)).sum::<f32>().sqrt();

        dot_product / (magnitude1 * magnitude2)
    }

    pub fn semantic_vector(&mut self, phrases: Vec<&str>) -> Vec<f32> {
        let mut sum_vector = vec![0.0; self.embeddings.dims()];
        let mut count = 0;

        for phrase in phrases {
            let vector = self.average_vector(phrase);
            for i in 0..vector.len() {
                sum_vector[i] += vector[i];
            }
            count += 1;
        }

        for i in 0..sum_vector.len() {
            sum_vector[i] /= count as f32;
        }

        sum_vector
    }

    pub(crate) fn similarity_string(&mut self, phrase1: &str, phrase2: &str) -> f32 {
        let vector1 = self.average_vector(phrase1);
        let vector2 = self.average_vector(phrase2);

        Embedding::cosine_similarity(&vector1, &vector2)
    }
}