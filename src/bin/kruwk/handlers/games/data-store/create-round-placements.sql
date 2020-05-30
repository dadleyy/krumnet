insert into
  krumnet.game_member_round_placement_results as placements
  (user_id, lobby_id, member_id, game_id, round_id, place)
select
  entries.user_id      as submitter,
  entries.lobby_id     as lobby_id,
  entries.member_id    as member_id,
  entries.game_id      as game_id,
  entries.round_id     as round_id,
  row_number() over () as placement
from
  krumnet.game_round_entry_votes as votes
left join
  krumnet.game_round_entries as entries
on
  entries.id = votes.entry_id
where
  votes.round_id = $1
group by
  entries.id
order by
  count(votes.id) desc
on conflict on constraint
  single_round_winner
do update set
  created_at = now();
