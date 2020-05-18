with new_game as (
  insert into krumnet.games as games
    (lobby_id, name, job_id)
  values
    ($1, $2, $3)
  returning
    id
) insert into krumnet.game_rounds
    (position, game_id, started_at)
  select
    new_rounds.position, new_game.id, new_rounds.started
  from
    (select
        ser as position, started
      from
        generate_series(0, 3) as ser
      left join
        (select
          nums.position,
          nums.started
        from
          (select
            row_number() over () as position,
            v as started
          from
            generate_series(now() + interval '1 minute', now() + interval '1 minute', '1 minute')
          as v)
        as nums) as starts
      on ser = starts.position - 1
    ) as new_rounds,
    new_game
  returning
    game_id;
