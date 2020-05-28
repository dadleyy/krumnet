update
  krumnet.game_rounds
set
  started_at = now()
where
  krumnet.game_rounds.game_id = $1
and
  krumnet.game_rounds.position = $2 + 1
returning
  id;
