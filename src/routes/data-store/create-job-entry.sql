with new_entry as (
  insert into
    krumnet.game_round_entries
    (round_id, member_id, entry)
  values
    ($1, $2, $3)
  returning *
) select id, entry, round_id from new_entry;
