select
  members.id,
  users.id,
  users.default_email,
  users.name,
  members.invited_by,
  members.joined_at,
  members.left_at,
  members.permissions
from
  krumnet.lobby_memberships as members
inner join
  krumnet.users as users on users.id = members.user_id
where
  members.lobby_id = $1;
