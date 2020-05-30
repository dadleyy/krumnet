select
  games.lobby_id      as lobby_id,
  games.id            as game_id,
  rounds.id           as round_id,
  memberships.id      as member_id,
  memberships.user_id as user_id
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
