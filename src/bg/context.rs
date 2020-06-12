use crate::{JobStore, RecordStore};
use async_std::sync::Arc;

pub struct Context {
  pub records: Arc<RecordStore>,
  pub jobs: Arc<JobStore>,
}
