select
  lobbies.id                     as lobby_id,
  lobbies.name                   as lobby_name,
  lobbies.created_at             as created_at,
  count(distinct memberships.id) as member_count,
  count(distinct games.id)       as game_count
from
  krumnet.lobbies as lobbies
left join
  krumnet.lobby_memberships as memberships
on
  lobbies.id = memberships.lobby_id
left join
  krumnet.games as games
on
  games.lobby_id = lobbies.id
where
  lobbies.id in (
    select
      lobby_id
    from
      krumnet.lobby_memberships as m
    where
      m.user_id = $1
    and
      m.left_at is null
    and
      m.joined_at is not null
    limit 10
  )
and
  memberships.left_at is null
group by
  lobbies.id, memberships.lobby_id
order by
  lobbies.created_at desc;
