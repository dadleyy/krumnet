select
  entries.id as id
from
  krumnet.game_round_entries as entries
where
  entries.id = $1
and 
  entries.user_id <> $2;
