use log::info;
use std::io::Result;

use crate::http::{Response, StatusCode};
use crate::{authorization, errors, Authorization, AuthorizationUrls, RecordStore};

pub async fn provision(
  auth: &Option<Authorization>,
  _records: &RecordStore,
  urls: &AuthorizationUrls,
) -> Result<Response<()>> {
  let builder = authorization::cors_builder(urls)?;

  let uid = match auth {
    Some(Authorization(id, _name, _email, _token)) => id,
    None => {
      return builder
        .status(StatusCode::NOT_FOUND)
        .body(())
        .map(|r| Response::json(r))
        .map_err(errors::humanize_error)
    }
  };

  info!("attempt to provision lobby from user '{}'", uid);

  builder
    .status(StatusCode::OK)
    .body(())
    .map(|r| Response::json(r))
    .map_err(errors::humanize_error)
}
