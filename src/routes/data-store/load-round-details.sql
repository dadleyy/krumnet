select
  rounds.id,
  rounds.prompt,
  rounds.position,
  rounds.created_at,
  rounds.completed_at,
  rounds.started_at
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
