select
  members.id,
  members.permissions,
  members.created_at,
  users.id,
  users.default_email,
  users.name
from
  krumnet.game_memberships as members
right join
  krumnet.users as users
on
  users.id = members.user_id
where
  members.game_id = $1;
