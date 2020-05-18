select
  game.id,
  game.created_at,
  game.name,
  count(member.id)
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
