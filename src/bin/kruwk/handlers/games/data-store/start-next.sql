update
  krumnet.game_rounds as rounds
set
  started_at = now()
where
  rounds.game_id = $1
and
  rounds.position = $2 + 1
returning
  id;
