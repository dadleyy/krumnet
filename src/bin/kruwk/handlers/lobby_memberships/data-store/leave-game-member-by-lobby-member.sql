update
  krumnet.game_memberships as game_memberships
set
  left_at = now()
where
  game_memberships.lobby_member_id = $1
returning
  game_memberships.game_id,
  game_memberships.lobby_id,
  game_memberships.id,
  game_memberships.lobby_member_id,
  game_memberships.user_id;
