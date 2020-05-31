select
  placements.id    as id,
  placements.place as placement,
  users.name       as user_name,
  users.id         as user_id
from
  krumnet.game_member_placement_results as placements
left join
  krumnet.users as users
on
  users.id = placements.user_id
where
  placements.game_id = $1;
