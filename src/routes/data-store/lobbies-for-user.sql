select
  lobbies.id,
  lobbies.name,
  lobbies.created_at,
  count(games.id),
  count(memberships.id)
from
  krumnet.lobby_memberships as memberships
right join
  krumnet.lobbies as lobbies
on
  lobbies.id = memberships.lobby_id
left join
  krumnet.games as games
on
  games.lobby_id = lobbies.id
where
  memberships.user_id = $1
group by
  lobbies.id
limit 10;
