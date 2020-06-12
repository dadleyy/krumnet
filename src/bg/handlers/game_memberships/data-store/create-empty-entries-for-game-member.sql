insert into
  krumnet.game_round_entries
  (user_id, round_id, member_id, game_id, lobby_id, entry, auto)
select
  cast($1 as varchar),
  rounds.id,
  cast($2 as varchar),
  rounds.game_id,
  rounds.lobby_id,
  '',
  true
from
  krumnet.game_rounds as rounds
where
  rounds.id = any($3)
returning
  id,
  game_id,
  round_id;
