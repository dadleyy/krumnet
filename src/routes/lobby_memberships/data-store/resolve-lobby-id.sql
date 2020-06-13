select
  lobbies.id as id
from
  krumnet.lobbies as lobbies
where
  lobbies.name like $1
limit 1;
