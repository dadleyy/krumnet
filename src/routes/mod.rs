use log::info;
use sqlx::query_file;
use std::io::Result;

pub mod games;
pub mod jobs;
pub mod lobbies;
pub mod lobby_memberships;
pub mod rounds;

use crate::http::{query as qs, Uri};
use crate::interchange::http::{SessionData, SessionUserData};
use crate::{errors, Authority, Context, Response};

pub async fn destroy(context: &Context, uri: &Uri) -> Result<Response> {
  let token = match context.authority() {
    Authority::User { id: _, token } => Some(token.clone()),

    Authority::None => uri
      .query()
      .and_then(|q| qs::parse(q.as_bytes()).find(|(k, _k)| k == "token"))
      .map(|(_k, v)| String::from(v.as_ref())),
  }
  .unwrap_or_default();

  info!("destroying session from token: {}", token);
  context.session().destroy(&token).await?;

  Ok(Response::redirect(&context.config().krumi.auth_uri))
}

pub async fn identify(context: &Context) -> Result<Response> {
  let uid = match context.authority() {
    Authority::User { id, token: _ } => id,
    Authority::None => return Ok(Response::not_found().cors(context.cors())),
  };

  info!("loading sesison for user {}", uid);
  let mut conn = context.records_connection().await?;

  query_file!("src/data-store/user-for-session.sql", uid)
    .fetch_all(&mut conn)
    .await
    .map_err(errors::humanize_error)?
    .into_iter()
    .nth(0)
    .map(|row| SessionData {
      user: SessionUserData {
        id: row.user_id,
        name: row.user_name,
        email: row.user_email,
      },
    })
    .ok_or_else(|| errors::e("Not found"))
    .and_then(|tenant| Response::ok_json(&tenant).map(|r| r.cors(context.cors())))
}
