select
  rounds.id           as round_id,
  rounds.prompt       as prompt,
  rounds.position     as pos,
  rounds.created_at   as created_at,
  rounds.fulfilled_at as fulfilled_at,
  rounds.completed_at as completed_at,
  rounds.started_at   as started_at
from
  krumnet.game_rounds as rounds
right join
  krumnet.game_memberships as members
on
  members.game_id = rounds.game_id
where
  rounds.id = $2
and
  members.user_id = $1;
