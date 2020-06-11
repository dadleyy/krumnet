pub mod context;
pub mod handlers;

#[cfg(test)]
pub mod test_helpers {
  use crate::{
    bg::context::Context, configuration::test_helpers::load_test_config, JobStore, RecordStore,
  };
  use async_std::sync::Arc;

  pub async fn get_test_context() -> Context {
    let config = load_test_config().expect("unable to load test config");

    let records = RecordStore::open(&config)
      .await
      .expect("unable to open record store");

    let jobs = JobStore::open(&config)
      .await
      .expect("unable to open job store");

    Context {
      records: Arc::new(records),
      jobs: Arc::new(jobs),
    }
  }
}
