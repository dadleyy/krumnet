use crate::{
  http::{query as qs, Uri},
  interchange::http::JobHandle,
  interchange::jobs::QueuedJob,
  Authority, Context, Response,
};
use log::debug;
use std::io::Result;

fn with_access(auth: &Authority, job: QueuedJob) -> Option<QueuedJob> {
  match auth {
    Authority::User { id, token: _ } => job.user().and_then(|job_user| {
      if &job_user == id {
        debug!("job '{}' owned by '{}', we good", job.id, job_user);
        return Some(job);
      }

      debug!("job '{}' not owned by '{}', we good", job.id, job_user);
      None
    }),
    Authority::None => None,
  }
}

pub async fn find(context: &Context, uri: &Uri) -> Result<Response> {
  let uid = match context.authority() {
    Authority::User { id, token: _ } => id,
    Authority::None => return Ok(Response::not_found().cors(context.cors())),
  };

  debug!("user '{}' is requesting access to job", uid);

  let id = uri
    .query()
    .and_then(|q| qs::parse(q.as_bytes()).find(|(k, _k)| k == "id"))
    .map(|(_k, v)| String::from(v.as_ref()));

  match id {
    Some(id) => {
      debug!("found job id '{}' searching queue", id);
      let job = context.jobs().lookup(&id).await?;

      match job {
        Some(job) => {
          debug!("job '{}' found, validing creator", job.id);

          with_access(context.authority(), job)
            .map(|job| {
              debug!("user has access to job");
              Response::ok_json(JobHandle::from(job)).map(|r| r.cors(context.cors()))
            })
            .unwrap_or(Ok(Response::not_found().cors(context.cors())))
        }
        None => {
          debug!("job '{}' not found", id);
          Ok(Response::default().cors(context.cors()))
        }
      }
    }
    None => {
      debug!("no job id found in query string for {:?}", uri);
      Ok(Response::not_found().cors(context.cors()))
    }
  }
}

#[cfg(test)]
mod test {
  use super::with_access;
  use crate::{
    interchange::jobs::{CreateLobby, Job, QueuedJob},
    Authority,
  };

  #[test]
  fn auth_none() {
    let uid = String::from("s-123");
    let job = QueuedJob {
      id: String::from("s-job"),
      job: Job::CreateLobby(CreateLobby {
        creator: uid.clone(),
        result: None,
      }),
    };
    let auth = Authority::None;
    assert!(with_access(&auth, job).is_none());
  }

  #[test]
  fn auth_user_without_access() {
    let uid = String::from("s-123");
    let job = QueuedJob {
      id: String::from("s-job"),
      job: Job::CreateLobby(CreateLobby {
        creator: format!("{}-456", uid.clone()),
        result: None,
      }),
    };
    let auth = Authority::User {
      id: uid.clone(),
      token: String::from(""),
    };
    assert!(with_access(&auth, job).is_none());
  }

  #[test]
  fn auth_user_with_access() {
    let uid = String::from("s-123");
    let job = QueuedJob {
      id: String::from("s-job"),
      job: Job::CreateLobby(CreateLobby {
        creator: uid.clone(),
        result: None,
      }),
    };
    let auth = Authority::User {
      id: uid.clone(),
      token: String::from(""),
    };
    assert!(with_access(&auth, job).is_some());
  }
}
