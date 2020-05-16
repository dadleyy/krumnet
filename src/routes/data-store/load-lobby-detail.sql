select
  lobbies.id,
  lobbies.name,
  lobbies.settings,
  lobbies.created_at,
  count(members.*)
from
  krumnet.lobbies as lobbies
inner join krumnet.lobby_memberships as members on members.lobby_id = lobbies.id
where
  lobbies.id = $1
and
  members.user_id = $2
and
  members.lobby_id = $1
group by lobbies.id;
