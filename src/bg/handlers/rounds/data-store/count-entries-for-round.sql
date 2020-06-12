select
  rounds.id         as round_id,
  count(entries.id) as entry_count
from
  krumnet.game_rounds as rounds
left join
  krumnet.game_round_entries as entries
on
  rounds.id = entries.round_id
where
  rounds.id = $1
group by
  rounds.id;
