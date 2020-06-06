select
  rounds.id           as id,
  rounds.position     as pos,
  rounds.prompt       as prompt,
  rounds.created_at   as created_at,
  rounds.started_at   as started_at,
  rounds.completed_at as completed_at,
  rounds.fulfilled_at as fulfilled_at
from
  krumnet.game_rounds as rounds
where
  rounds.game_id = $1;
