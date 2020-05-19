select
  entries.id,
  entries.round_id,
  entries.member_id,
  entries.entry,
  entries.created_at,
  users.id,
  users.name,
  users.default_email
from
  krumnet.game_round_entries as entries
left join
  krumnet.game_rounds as rounds
on
  rounds.id = entries.round_id
left join
  krumnet.game_memberships as memberships
left join
  krumnet.users as users
on
  users.id = memberships.user_id
on
  memberships.game_id = rounds.game_id
where
  memberships.user_id = $1
and
  rounds.id = $2;
