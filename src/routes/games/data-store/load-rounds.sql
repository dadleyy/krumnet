select
  rounds.id,
  rounds.position,
  rounds.prompt,
  rounds.created_at,
  rounds.started_at,
  rounds.completed_at
from
  krumnet.game_rounds as rounds
where
  rounds.game_id = $1;
