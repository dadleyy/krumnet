use log::info;
use std::io::Result;

use crate::http::{Response, StatusCode};
use crate::{authorization, errors, Context};

pub async fn provision(context: &Context<'_>) -> Result<Response<()>> {
  let builder = authorization::cors_builder(context.urls())?;

  let uid = match context.auth() {
    Some(authorization::Authorization(id, _name, _email, _token)) => id,
    None => {
      return builder
        .status(StatusCode::NOT_FOUND)
        .body(())
        .map(|r| Response::json(r))
        .map_err(errors::humanize_error)
    }
  };

  info!("attempting to provision lobby for user '{}'", uid);

  Ok(Response::not_found(None))
}
