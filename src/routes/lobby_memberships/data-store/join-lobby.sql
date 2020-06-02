insert into
  krumnet.lobby_memberships (lobby_id, user_id, joined_at)
select
  lobbies.id, cast($2 as varchar), now()
from
  krumnet.lobbies as lobbies
left join
  krumnet.lobby_memberships as memberships
on
  memberships.lobby_id = lobbies.id
where
  lobbies.closed_at is null
and
  lobbies.name = trim(from $1)
or
  lobbies.id = trim(from $1)
group by
  lobbies.id
having
  sum(case when memberships.user_id = $2 and memberships.left_at is null then 1 else 0 end) = 0
on conflict on constraint
  single_membership
do update set
  left_at = null, joined_at = now()
returning
  id       as member_id,
  lobby_id as lobby_id,
  user_id  as user_id;
