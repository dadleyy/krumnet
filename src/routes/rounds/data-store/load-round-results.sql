select
  results.id      as result_id,
  results.user_id as user_id,
  users.name      as user_name,
  results.place   as round_place
from
  krumnet.game_member_round_placement_results as results
left join
  krumnet.users as users
on
  users.id = results.user_id
where
  results.round_id = $1;
