use rand::Rng;
use uuid::Uuid;


pub trait IdGenerator {
    fn gen(&self) -> String;
}

#[derive(Clone)]
pub struct UuidGenerator {
}

impl UuidGenerator {
    pub fn new() -> Self {
        Self {
        }
    }
}

impl IdGenerator for UuidGenerator {
    fn gen(&self) -> String {
        Uuid::new_v4().to_string()
    }
}

#[derive(Clone)]
pub struct ShortIdGenerator {
}

impl ShortIdGenerator {
    pub fn new() -> Self {
        Self {
        }
    }
}

impl IdGenerator for ShortIdGenerator {
    fn gen(&self) -> String {
        let mut rng = rand::thread_rng();
        let possible = "0123456789abcdefghijkmnpqrstuvwxyz";

        let mut random_char = || {
            let rand_index = rng.gen_range(0, possible.len());
            possible.chars().nth(rand_index).expect("error generating index")
        };

        let mut id = String::new();

        for _ in 0..4 {
            id.push(random_char());
        }

        id.push('-');

        for _ in 0..4 {
            id.push(random_char());
        }

        id
    }
}

pub fn create_generator(gen_type: &str) -> Box<IdGenerator + Send + Sync> {
    if gen_type == "uuid" {
        Box::new(UuidGenerator::new())
    }
    else {
        Box::new(ShortIdGenerator::new())
    }
}
