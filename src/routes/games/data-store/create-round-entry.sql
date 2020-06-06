with new_entry as (
  insert into
    krumnet.game_round_entries
    (round_id, member_id, entry, game_id, lobby_id, user_id)
  values
    ($1, $2, $3, $4, $5, $6)
  returning *
) select
  id       as entry_id,
  entry    as entry,
  round_id as round_id
from
  new_entry;
