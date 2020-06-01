update
  krumnet.game_rounds as rounds
set
  completed_at = now()
where
  rounds.id = $1
returning
  position,
  game_id;
