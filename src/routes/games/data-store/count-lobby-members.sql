select
  count(*) as member_count
from
  krumnet.lobby_memberships as members
where
  members.lobby_id = $1
and
  members.left_at is null;
