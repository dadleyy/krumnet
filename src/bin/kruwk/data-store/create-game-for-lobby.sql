with new_game as (
  insert into krumnet.games as games
    (lobby_id, job_id)
  values
    ($1, $2)
  returning
    id
) insert into krumnet.game_rounds
    (position, game_id)
  select
    round_numbers, new_game.id
  from
    generate_series(0, 2) as round_numbers, new_game
  returning
    game_id;
