select
  count(members.id)
from
  krumnet.game_rounds as rounds
left join
  krumnet.game_memberships as members
on
  rounds.game_id = members.game_id
where
  rounds.id = $1;
