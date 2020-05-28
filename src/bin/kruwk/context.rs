use krumnet::{JobStore, RecordStore};

pub struct Context<'a> {
  pub records: &'a RecordStore,
  pub jobs: &'a JobStore,
}
