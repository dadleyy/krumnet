insert into
  krumnet.game_member_placement_results as game_placements
  (user_id, lobby_id, member_id, game_id, place)
select
  round_placements.user_id    as user_id,
  round_placements.lobby_id   as lobby_id,
  round_placements.member_id  as member_id,
  round_placements.game_id    as game_id,
  row_number() over ()        as placement
from
  krumnet.game_member_round_placement_results as round_placements
where
  round_placements.game_id = $1
group by
  round_placements.game_id,
  round_placements.lobby_id,
  round_placements.user_id,
  round_placements.member_id
order by
  sum(round_placements.place) asc
on conflict on constraint
  single_game_winner
do update set
  created_at = now()
returning
  id,
  game_id;
