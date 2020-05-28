select
  count(members.id)
from
  krumnet.lobbies as lobbies
left join
  krumnet.lobby_memberships as members
on
  members.lobby_id = lobbies.id
where
  members.left_at is null
and
  lobbies.id = $1;
