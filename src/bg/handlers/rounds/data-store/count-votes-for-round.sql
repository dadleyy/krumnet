select
  count(votes.id) as count
from
  krumnet.game_round_entry_votes as votes
where
  votes.round_id = $1;
