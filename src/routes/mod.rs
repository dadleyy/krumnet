use log::info;
use std::io::Result;

pub mod lobbies;

use crate::http::{query as qs, Uri};
use crate::interchange::http::{SessionData, SessionUserData};
use crate::records::Row;
use crate::{Authority, Context, Response};

const USER_FOR_SESSION: &'static str = include_str!("../data-store/load-user-for-session.sql");

pub fn parse_user_session_query(row: Row) -> Option<SessionUserData> {
  let id = row.try_get(0).ok()?;
  let name = row.try_get(1).ok()?;
  let email = row.try_get(2).ok()?;
  Some(SessionUserData { id, email, name })
}

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

  let tenant = context
    .records()
    .query(USER_FOR_SESSION, &[&uid])
    .ok()
    .and_then(|mut rows| rows.pop())
    .and_then(parse_user_session_query)
    .map(|user| SessionData { user });

  info!("loaded sesison data for '{}' (payload {:?})", uid, tenant);
  Response::ok_json(&tenant).map(|r| r.cors(context.cors()))
}
