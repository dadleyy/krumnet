select
  lobbies.id,
  lobbies.name,
  lobbies.created_at,
  count(distinct memberships.id) member_count,
  count(distinct games.id) game_count
from
  krumnet.lobbies as lobbies
left join
  krumnet.lobby_memberships as memberships
on
  lobbies.id = memberships.lobby_id
left join
  krumnet.games as games
on
  games.lobby_id = lobbies.id
where
  memberships.user_id = $1
and
  memberships.left_at is null
group by
  lobbies.id, memberships.lobby_id
order by
  lobbies.created_at desc
limit 10;
