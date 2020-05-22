with new_game as (
  insert into krumnet.games as games
    (lobby_id, name, job_id)
  values
    ($1, $2, $3)
  returning
    id
) insert into krumnet.game_rounds
    (position, game_id, prompt, started_at)
  select
    new_rounds.position, new_game.id, new_rounds.prompt, new_rounds.started_at
  from
    (
      select
        positions.position, numbered_prompts.prompt, starts.started_at
      from 
        generate_series(0, 2) as positions (position)
      left join
        (
          select
            prompts.prompt, row_number() over () i
          from
            krumnet.prompts as prompts tablesample BERNOULLI (10)
          limit 3
        ) as numbered_prompts
      on
        numbered_prompts.i - 1 = positions.position
      left join
        (values (0, now())) as starts (j, started_at)
      on
        starts.j = positions.position
    ) as new_rounds,
    new_game
  returning
    game_id;
