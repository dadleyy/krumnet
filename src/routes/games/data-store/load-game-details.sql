select
  game.id           as game_id,
  game.created_at   as created_at,
  game.name         as game_name,
  game.ended_at     as ended_at,
  count(member.id)  as member_count
from
  krumnet.games as game
inner join
  krumnet.game_memberships as member
on
  member.game_id = game.id
where
  game.id = $1
and
  member.user_id = $2
group by
  game.id;
