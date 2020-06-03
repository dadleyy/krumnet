
exports.up = async function(knex) {
  await knex.schema.withSchema('krumnet').table('game_member_round_placement_results', function(table) {
    table.integer('vote_count').defaultTo(0).notNullable();
  });
  await knex.schema.withSchema('krumnet').table('game_member_placement_results', function(table) {
    table.integer('vote_count').defaultTo(0).notNullable();
  });
};

exports.down = async function(knex) {
  await knex.schema.withSchema('krumnet').table('game_member_round_placement_results', function(table) {
    table.dropColumn('vote_count');
  });
  await knex.schema.withSchema('krumnet').table('game_member_placement_results', function(table) {
    table.integer('vote_count').defaultTo(0).notNullable();
  });
};
