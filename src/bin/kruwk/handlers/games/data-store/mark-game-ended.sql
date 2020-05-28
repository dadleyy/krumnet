update
  krumnet.games as games
set
  ended_at = now()
where
  games.id = $1;
