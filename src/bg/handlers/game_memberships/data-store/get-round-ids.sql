select
  rounds.id as round_id
from
  krumnet.game_rounds as rounds
left join
  krumnet.game_round_entries as entries
on
  entries.round_id = rounds.id
and
  entries.user_id = $1
where
  rounds.game_id = $2
group by
  rounds.id
having
  count(entries.id) = 0;
