select
  entries.id,
  entries.round_id,
  entries.member_id,
  entries.entry,
  entries.created_at,
  entries.user_id
from
  krumnet.game_round_entries as entries
where
  entries.round_id = $1;
