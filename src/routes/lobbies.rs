use std::io::Result;

use log::info;

use crate::http::Response;
use crate::interchange::http::ProvisioningAttemptHandle;
use crate::interchange::provisioning::{ProvisioningAttempt, ProvisioningAttemptAuthority};
use crate::{authorization, errors, StaticContext};

pub async fn provision(context: &StaticContext) -> Result<Response<ProvisioningAttemptHandle>> {
  let builder = authorization::cors_builder(context.urls())?;

  let uid = match context.auth() {
    Some(authorization::Authorization(id, _name, _email, _token)) => id,
    None => {
      return Ok(Response::not_found(
        authorization::cors(context.urls()).ok(),
      ))
    }
  };

  let authority = ProvisioningAttemptAuthority::User { id: uid.clone() };
  let attempt = ProvisioningAttempt::Lobby { authority };
  let result = context.records().queue(attempt).await?;
  info!("command result: {:?}", result);

  builder
    .body(ProvisioningAttemptHandle { id: result })
    .map(|r| Response::json(r))
    .map_err(errors::humanize_error)
}

#[cfg(test)]
mod test {

  #[test]
  fn test_unauthorized() {
    assert!(true);
  }
}
