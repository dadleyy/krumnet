select
  members.id member_id,
  members.permissions permissions,
  members.created_at created_at,
  users.id user_id,
  users.default_email user_email,
  users.name user_name
from
  krumnet.game_memberships as members
right join
  krumnet.users as users
on
  users.id = members.user_id
where
  members.game_id = $1;
