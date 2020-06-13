insert into
  krumnet.game_round_entry_votes
  (round_id, lobby_id, game_id, member_id, user_id, entry_id)
select
  entries.round_id,
  entries.lobby_id,
  entries.game_id,
  cast($2 as varchar),
  cast($3 as varchar),
  entries.id
from
  krumnet.game_round_entries as entries
where
  entries.id = $1
returning
  id;
