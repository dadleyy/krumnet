insert into
  krumnet.game_member_round_placement_results as placements
  (user_id, lobby_id, member_id, game_id, round_id, place, vote_count)
select
  entries.user_id                                   as submitter,
  entries.lobby_id                                  as lobby_id,
  entries.member_id                                 as member_id,
  entries.game_id                                   as game_id,
  entries.round_id                                  as round_id,
  row_number() over (order by count(votes.id) desc) as placement,
  count(votes.id)                                   as vote_count
from
  krumnet.game_round_entries as entries
left join
  krumnet.game_round_entry_votes as votes
on
  entries.id = votes.entry_id
where
  entries.round_id = $1
group by
  entries.id
order by
  count(votes.id) desc
on conflict on constraint
  single_round_winner
do update set
  created_at = now()
returning
  id;
