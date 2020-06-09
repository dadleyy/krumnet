select
  count(members.id) as member_count
from
  krumnet.lobby_memberships as members
where
  members.lobby_id = $1;
