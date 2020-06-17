select
  count(votes.id) as count
from
  krumnet.game_round_entry_votes as votes
left join
  krumnet.game_rounds as rounds
on
  votes.round_id = rounds.id
where
  rounds.id = $1
and
  rounds.fulfilled_at is not null;
