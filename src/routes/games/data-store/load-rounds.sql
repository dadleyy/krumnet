select
  rounds.id id,
  rounds.position pos,
  rounds.prompt prompt,
  rounds.created_at created_at,
  rounds.started_at started_at,
  rounds.completed_at completed_at,
  rounds.fulfilled_at fulfilled_at
from
  krumnet.game_rounds as rounds
where
  rounds.game_id = $1;
