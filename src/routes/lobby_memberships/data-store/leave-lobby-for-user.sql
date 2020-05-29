update
  krumnet.lobby_memberships as memberships
set
  left_at = now()
where
  memberships.lobby_id = $1
and
  memberships.user_id = $2
returning
  memberships.id       as member_id,
  memberships.lobby_id as lobby_id;
