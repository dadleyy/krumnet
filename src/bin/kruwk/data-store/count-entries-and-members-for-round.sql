select
  count(members.id),
  count(entries.id)
from
  krumnet.game_rounds as rounds
inner join
  krumnet.game_round_entries as entries
on
  entries.round_id = rounds.id
inner join
  krumnet.game_memberships as members
on
  members.game_id = rounds.game_id
where
  rounds.id = $1
group by
  rounds.id;
