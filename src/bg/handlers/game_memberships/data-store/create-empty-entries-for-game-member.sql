insert into
  krumnet.game_round_entries
  (user_id, round_id, member_id, game_id, lobby_id, entry, auto)
select
  cast($1 as varchar),
  rounds.id,
  members.id,
  rounds.game_id,
  rounds.lobby_id,
  '',
  true
from
  krumnet.game_rounds as rounds
left join
  krumnet.game_round_entries as entries
on
  entries.round_id = rounds.id
left join
  krumnet.game_memberships as members
on
  members.game_id = rounds.game_id
where
  members.user_id = $1
group by
  rounds.id, members.id
having
  count(entries.id) = 0
order by
  rounds.position asc
returning
  id,
  game_id,
  round_id;
