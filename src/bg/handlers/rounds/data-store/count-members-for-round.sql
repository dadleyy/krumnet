select
  rounds.id         as round_id,
  count(members.id) as member_count
from
  krumnet.game_rounds as rounds
left join
  krumnet.game_memberships as members
on
  rounds.game_id = members.game_id
where
  rounds.id = $1
group by
  rounds.id;
