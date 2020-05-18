select
  games.id,
  games.created_at,
  games.name,
  count(rounds.id)
from
  krumnet.games as games
right join
  krumnet.game_rounds as rounds
on 
  rounds.game_id = games.id
where
  games.lobby_id = $1
and
  rounds.completed_at is null
group by
  games.id
order by
  games.created_at desc
limit
  10;
