select
  entries.id          as entry_id,
  entries.round_id    as round_id,
  entries.member_id   as member_id,
  entries.created_at  as created_at,
  entries.user_id     as user_id,
  users.name          as user_name,
  entries.entry       as entry,
  rounds.fulfilled_at as fulfilled
from
  krumnet.game_round_entries as entries
left join
  krumnet.game_rounds as rounds
on
  rounds.id = entries.round_id
left join
  krumnet.users as users
on
  users.id = entries.user_id
where
  entries.round_id = $1;
