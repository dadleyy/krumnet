select
  games.lobby_id,
  games.id,
  rounds.id,
  memberships.id,
  memberships.user_id
from
  krumnet.game_memberships as memberships
inner join
  krumnet.games as games
on
  memberships.game_id = games.id
inner join
  krumnet.game_rounds as rounds
on
  rounds.game_id = games.id
where
  rounds.id = $1
and
  memberships.user_id = $2;
