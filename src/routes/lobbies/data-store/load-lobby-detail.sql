select
  lobbies.id          as lobby_id,
  lobbies.name        as lobby_name,
  lobbies.created_at  as created_at,
  count(members.*)    as member_count
from
  krumnet.lobbies as lobbies
inner join
  krumnet.lobby_memberships as members
on
  members.lobby_id = lobbies.id
where
  lobbies.id = $1
and
  members.user_id = $2
and
  members.lobby_id = $1
group by
  lobbies.id;
