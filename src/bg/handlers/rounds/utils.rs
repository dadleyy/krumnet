use crate::bg::context::Context;
use log::warn;
use sqlx::query_file;

fn warn_and_stringify<E: std::error::Error>(e: E) -> String {
  warn!("{}", e);
  format!("{}", e)
}

pub async fn count_members(round_id: &String, context: &Context) -> Result<i64, String> {
  let mut conn = context
    .records
    .acquire()
    .await
    .map_err(warn_and_stringify)?;

  query_file!(
    "src/bg/handlers/rounds/data-store/count-members-for-round.sql",
    round_id
  )
  .fetch_all(&mut conn)
  .await
  .map_err(warn_and_stringify)?
  .into_iter()
  .nth(0)
  .and_then(|row| row.member_count)
  .ok_or(format!("Unable to count members for round '{}'", round_id))
}
