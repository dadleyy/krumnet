select
  members.id          as member_id,
  users.id            as user_id,
  users.default_email as user_email,
  users.name          as user_name,
  members.invited_by  as invited_by,
  members.joined_at   as joined_at,
  members.left_at     as left_at
from
  krumnet.lobby_memberships as members
inner join
  krumnet.users as users on users.id = members.user_id
where
  members.lobby_id = $1;
