update
  krumnet.lobbies as lobbies
set
  closed_at = now()
where
  lobbies.id = $1;
