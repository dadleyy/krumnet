select
  rounds.id,
  rounds.position,
  rounds.completed_at
from
  krumnet.game_rounds as rounds
where
  rounds.game_id = $1;
