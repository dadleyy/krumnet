select
  members.id          as member_id,
  members.created_at  as created_at,
  users.id            as user_id,
  users.default_email as user_email,
  users.name          as user_name
from
  krumnet.game_memberships as members
right join
  krumnet.users as users
on
  users.id = members.user_id
where
  members.game_id = $1;
