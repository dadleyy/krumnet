with new_entry as (
  insert into
    krumnet.game_round_entries
    (round_id, member_id, entry, game_id, lobby_id)
  values
    ($1, $2, $3, $4, $5)
  returning *
) select id, entry, round_id from new_entry;
