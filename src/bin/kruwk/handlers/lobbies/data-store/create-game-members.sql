insert into krumnet.game_memberships
  (user_id, game_id, lobby_id, lobby_member_id, permissions)
select
  m.user_id, $1, m.lobby_id, m.id, m.permissions
from
  krumnet.lobby_memberships as m
where
  m.lobby_id = $2
and
  m.left_at is null;
