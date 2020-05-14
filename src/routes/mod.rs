use log::info;
use std::io::Result;

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

pub async fn identify(context: &Context) -> Result<Response> {
  let uid = match context.authority() {
    Authority::User(id) => id,
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
