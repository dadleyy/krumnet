use async_std::io::Read as AsyncRead;
use log::{debug, info, warn};
use serde::Deserialize;
use serde_json::from_slice as deserialize;
use sqlx::query_file;
use std::io::Result;
use std::marker::Unpin;

use crate::{constants, errors, interchange, read_size_async, Authority, Context, Response};

const TOO_MANY_MEMBERS: &'static str = "errors.lobbies.too_many_members";

#[derive(Deserialize, Debug)]
pub struct DestroyMembershipPayload {
  lobby_id: String,
}

async fn join_jobby(
  context: &Context,
  lobby_id: &String,
  user_id: &String,
) -> Result<(String, String, String)> {
  let mut conn = context.records_connection().await?;
  query_file!(
    "src/routes/lobby_memberships/data-store/join-lobby.sql",
    lobby_id,
    user_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(errors::humanize_error)?
  .into_iter()
  .nth(0)
  .map(|row| (row.member_id, row.lobby_id, row.user_id))
  .ok_or_else(|| errors::e("Unable to join lobby"))
}

async fn count_members(context: &Context, lobby_id: &String) -> Result<Option<i64>> {
  let mut conn = context.records_connection().await?;
  let result = query_file!(
    "src/routes/lobby_memberships/data-store/count-members.sql",
    lobby_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(errors::humanize_error)?
  .into_iter()
  .nth(0)
  .and_then(|row| row.member_count);

  Ok(result)
}

async fn replace_short_id(context: &Context, lobby_id: &String) -> Result<String> {
  if lobby_id.len() > 8 {
    return Ok(lobby_id.clone());
  }

  if lobby_id.len() < 5 {
    return Err(errors::e(format!("lobby id too short - '{}'", lobby_id)));
  }

  info!("attempting to resolve short id '{}'", lobby_id);
  let mut conn = context.records_connection().await?;
  query_file!(
    "src/routes/lobby_memberships/data-store/resolve-lobby-id.sql",
    format!("{}%", lobby_id.to_lowercase())
  )
  .fetch_all(&mut conn)
  .await
  .map_err(errors::humanize_error)?
  .into_iter()
  .nth(0)
  .map(|row| row.id)
  .ok_or(errors::e(format!("bad lobby id - '{}'", lobby_id)))
}

// Route
// POST /lobby-memberships
pub async fn create_membership<R>(context: &Context, reader: &mut R) -> Result<Response>
where
  R: AsyncRead + Unpin,
{
  let uid = match context.authority() {
    Authority::None => return Ok(Response::unauthorized().cors(context.cors())),
    Authority::User { id, .. } => id,
  };

  let contents = read_size_async(reader, context.pending()).await?;
  let payload = deserialize::<DestroyMembershipPayload>(&contents)?;
  let lobby_id = replace_short_id(context, &payload.lobby_id).await?;
  let member_count = count_members(context, &lobby_id).await?;

  match member_count {
    None => {
      warn!("unable to find lobby '{}' to join", lobby_id);
      return Ok(Response::not_found().cors(context.cors()));
    }
    Some(value) if value >= constants::MAX_LOBBY_MEMBERS.into() => {
      warn!("too many members in '{}' to join", lobby_id);
      return Ok(Response::bad_request(TOO_MANY_MEMBERS).cors(context.cors()));
    }
    Some(value) => info!("member count for '{}' satisfactory ({})", lobby_id, value),
  };

  let (member_id, lobby_id, user_id) = join_jobby(context, &lobby_id, &uid).await?;

  info!(
    "user {} is now member {} of lobby {}",
    user_id, member_id, lobby_id
  );
  let out = interchange::http::NewLobbyMembership {
    member_id,
    user_id,
    lobby_id,
  };
  Response::ok_json(&out).map(|r| r.cors(context.cors()))
}

async fn leave_lobby(
  context: &Context,
  lobby_id: &String,
  user_id: &String,
) -> Result<(String, String)> {
  let mut conn = context.records_connection().await?;
  query_file!(
    "src/routes/lobby_memberships/data-store/leave-lobby-for-user.sql",
    lobby_id,
    user_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(errors::humanize_error)?
  .into_iter()
  .nth(0)
  .map(|row| (row.member_id, row.lobby_id))
  .ok_or_else(|| errors::e("Unable to leave lobby"))
}

// Route
// DELETE /lobby-memberships
pub async fn destroy_membership<R>(context: &Context, reader: &mut R) -> Result<Response>
where
  R: AsyncRead + Unpin,
{
  let uid = match context.authority() {
    Authority::None => return Ok(Response::unauthorized().cors(context.cors())),
    Authority::User { id, .. } => id,
  };

  let contents = read_size_async(reader, context.pending()).await?;
  let payload = deserialize::<DestroyMembershipPayload>(&contents)?;

  debug!(
    "attempting to delete membership for user '{}', lobby '{}'",
    uid, payload.lobby_id
  );

  let (member_id, lobby_id) = leave_lobby(context, &payload.lobby_id, &uid).await?;

  if member_id.len() == 0 {
    warn!(
      "unable to find row to delete user[{}] lobby[{}]",
      uid, payload.lobby_id
    );
    return Ok(Response::not_found().cors(context.cors()));
  }

  info!("marking membership '{}' as left", member_id);
  let details = interchange::jobs::CleanupLobbyMembership {
    member_id,
    lobby_id,
    result: None,
  };

  context
    .jobs()
    .queue(&interchange::jobs::Job::CleanupLobbyMembership(details))
    .await?;

  Ok(Response::default().cors(context.cors()))
}

#[cfg(test)]
mod test {
  use super::replace_short_id;
  use crate::{
    bg,
    context::{test_helpers as context_helpers, Context},
    test_helpers::cleanup_lobby,
  };
  use async_std::task::block_on;
  use sqlx::query;

  async fn get_lobby_name(context: &Context, lobby_id: &String) -> String {
    let mut conn = context
      .records_connection()
      .await
      .expect("unable to connect");

    query!(
      "select name from krumnet.lobbies as lobbies where lobbies.id = $1",
      lobby_id
    )
    .fetch_all(&mut conn)
    .await
    .expect("unable to query")
    .into_iter()
    .nth(0)
    .map(|row| row.name)
    .expect("unable to get name")
  }

  #[test]
  fn resolve_lobby_id_short() {
    block_on(async {
      let job_id = "routes.lobby_memberships.resolve_lobby_id_short";
      let (ctx, user_id) = context_helpers::with_user_by_name(job_id).await;

      let lobby_id =
        bg::handlers::lobbies::make_lobby(ctx.records(), &job_id.to_string(), &user_id)
          .await
          .expect("unable to create");

      let mut name = get_lobby_name(&ctx, &lobby_id).await;
      let short_name = name.drain(0..5).collect::<String>();

      assert_eq!(replace_short_id(&ctx, &short_name).await.unwrap(), lobby_id);

      cleanup_lobby(&ctx, &lobby_id).await;
      context_helpers::cleanup(&ctx).await;
    });
  }

  #[test]
  fn resolve_lobby_id_full() {
    block_on(async {
      let job_id = "routes.lobby_memberships.resolve_lobby_id_full";
      let (ctx, user_id) = context_helpers::with_user_by_name(job_id).await;

      let lobby_id =
        bg::handlers::lobbies::make_lobby(ctx.records(), &job_id.to_string(), &user_id)
          .await
          .expect("unable to create");

      assert_eq!(replace_short_id(&ctx, &lobby_id).await.unwrap(), lobby_id);

      cleanup_lobby(&ctx, &lobby_id).await;
      context_helpers::cleanup(&ctx).await;
    });
  }

  #[test]
  fn resolve_lobby_id_garbage() {
    block_on(async {
      let job_id = "routes.lobby_memberships.resolve_lobby_id_garbage";
      let (ctx, _) = context_helpers::with_user_by_name(job_id).await;
      assert_eq!(
        replace_short_id(&ctx, &String::from("whoa")).await.is_err(),
        true
      );
      context_helpers::cleanup(&ctx).await;
    });
  }
}
