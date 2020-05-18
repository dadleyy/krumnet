insert into krumnet.game_memberships
  (user_id, game_id, permissions)
select
  m.user_id, $1, m.permissions
from
  krumnet.lobby_memberships as m
where
  m.lobby_id = $2;
