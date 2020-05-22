use krumnet::{interchange, RecordStore};
use log::debug;

pub async fn cleanup(
  job_id: &String,
  member_id: &String,
  records: &RecordStore,
) -> interchange::jobs::QueuedJob {
  debug!("job '{}', cleanup '{}'", job_id, member_id);

  interchange::jobs::QueuedJob {
    id: job_id.clone(),
    job: interchange::jobs::Job::CleanupLobbyMembership {
      member_id: member_id.clone(),
      result: None,
    },
  }
}
