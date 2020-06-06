select
  votes.id         as id,
  votes.entry_id   as entry_id,
  votes.member_id  as member_id,
  votes.user_id    as user_id,
  votes.created_at as created
from
  krumnet.game_round_entry_votes as votes
where
  votes.round_id = $1;
