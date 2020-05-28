select
  entries.id,
  entries.round_id,
  entries.member_id,
  entries.created_at,
  entries.user_id,
  users.name,
  entries.entry
from
  krumnet.game_round_entries as entries
left join
  krumnet.users as users
on
  users.id = entries.user_id
where
  entries.round_id = $1;
