with new_lobby as (
  insert into krumnet.lobbies
    (job_id, name)
  values
    ($1, $2)
  returning id
) insert into krumnet.lobby_memberships
    (user_id, lobby_id, joined_at)
  select
    $3, new_lobby.id, NOW()
  from
    new_lobby
  returning
    lobby_id;
