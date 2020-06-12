select
  count(members.id) as member_count
from
  krumnet.lobbies as lobbies
left join
  krumnet.lobby_memberships as members
on
  lobbies.id = members.lobby_id
where
  lobbies.name = $1
or
  lobbies.id = $1
and
  members.left_at is null;
