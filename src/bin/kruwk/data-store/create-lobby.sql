with new_lobby as (
  insert into lobbies (job_id, name, settings) values ($1, $2, $3) returning id
) insert into lobby_memberships (user_id, lobby_id, permissions) select $4, new_lobby.id, $3 from new_lobby;
