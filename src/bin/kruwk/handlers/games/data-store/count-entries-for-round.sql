select
  rounds.id         as round_id,
  count(entries.id) as entry_count
from
  krumnet.game_round_entries as entries
left join
  krumnet.game_rounds as rounds
on
  rounds.id = entries.round_id
where
  entries.round_id = $1
group by
  rounds.id;
