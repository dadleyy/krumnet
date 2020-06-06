select
  games.id          as game_id,
  games.created_at  as created_at,
  games.ended_at    as ended_at,
  games.name        as game_name,
  count(rounds.id)  as round_count
from
  krumnet.games as games
left join
  krumnet.game_rounds as rounds
on 
  rounds.game_id = games.id
and
  rounds.completed_at is null
where
  games.lobby_id = $1
group by
  games.id
order by
  games.created_at desc
limit
  10;
