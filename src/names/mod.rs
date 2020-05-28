use rand::prelude::*;
use rand::{distributions::Alphanumeric, rngs::ThreadRng, thread_rng};
use std::iter;

const NAMES: &'static str = include_str!("./data/names.txt");
const ADJECTIVES: &'static str = include_str!("./data/adjectives.txt");

fn rand_line(target: &'static str, rng: Option<ThreadRng>) -> (String, ThreadRng) {
  let items = target
    .split("\n")
    .map(|v| String::from(v))
    .collect::<Vec<String>>();
  let mut gen = rng.unwrap_or_else(thread_rng);
  let index = gen.next_u32() as usize % (items.len() - 1);
  (
    items
      .iter()
      .nth(index)
      .map(|s| s.clone())
      .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
    gen,
  )
}

pub fn get() -> String {
  let (ajective, mut rng) = rand_line(ADJECTIVES, None);
  let (name, _) = rand_line(NAMES, Some(rng));

  let buff = iter::repeat(())
    .map(|()| rng.sample(Alphanumeric))
    .take(5)
    .collect::<String>()
    .to_lowercase();

  format!("{}-{}-{}", ajective, name, buff)
}
