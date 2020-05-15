use rand::prelude::*;
use rand::thread_rng;

const NAMES: &'static str = include_str!("./data/names.txt");
const ADJECTIVES: &'static str = include_str!("./data/adjectives.txt");

fn rand_line(target: &'static str) -> String {
  let items = target
    .split("\n")
    .map(|v| String::from(v))
    .collect::<Vec<String>>();
  let mut gen = thread_rng();
  let index = gen.next_u32() as usize % (items.len() - 1);
  items
    .iter()
    .nth(index)
    .map(|s| s.clone())
    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

pub fn get() -> String {
  let (ajective, name) = (rand_line(ADJECTIVES), rand_line(NAMES));
  format!("{} {}", ajective, name)
}
