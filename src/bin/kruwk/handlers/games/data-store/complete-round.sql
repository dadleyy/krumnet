update
  krumnet.game_rounds
set
  completed_at = now()
where
  krumnet.game_rounds.id = $1
returning
  position, game_id;
