
exports.up = async function(knex) {
  await knex.schema.withSchema('krumnet').createTable('game_member_round_placement_results', function(table) {
    table.string('id', 36).defaultTo(knex.raw('uuid_generate_v4()')).notNullable().primary();
    table.string('user_id', 36).references('id').inTable('krumnet.users').notNullable();
    table.string('lobby_id', 36).references('id').inTable('krumnet.lobbies').notNullable();
    table.string('member_id', 36).references('id').inTable('krumnet.game_memberships').notNullable();
    table.string('game_id', 36).references('id').inTable('krumnet.games').notNullable();
    table.string('round_id', 36).references('id').inTable('krumnet.game_rounds').notNullable();
    table.timestamp('created_at').defaultTo(knex.fn.now());
    table.integer('place').unsigned().notNullable();
    table.unique('id');
    table.unique(['place', 'round_id'], 'single_round_winner');
    table.unique(['member_id', 'round_id'], 'single_member_round_placement');
  });
  await knex.schema.withSchema('krumnet').createTable('game_member_placement_results', function(table) {
    table.string('id', 36).defaultTo(knex.raw('uuid_generate_v4()')).notNullable().primary();
    table.string('user_id', 36).references('id').inTable('krumnet.users').notNullable();
    table.string('lobby_id', 36).references('id').inTable('krumnet.lobbies').notNullable();
    table.string('member_id', 36).references('id').inTable('krumnet.game_memberships').notNullable();
    table.string('game_id', 36).references('id').inTable('krumnet.games').notNullable();
    table.timestamp('created_at').defaultTo(knex.fn.now());
    table.integer('place').unsigned().notNullable();
    table.unique('id');
    table.unique(['place', 'game_id'], 'single_game_winner');
    table.unique(['member_id', 'game_id'], 'single_member_game_placement');
  });
};

exports.down = function(knex) {
  return Promise.all([
    knex.schema.withSchema('krumnet').dropTable('game_member_placement_results'),
    knex.schema.withSchema('krumnet').dropTable('game_member_round_placement_results'),
  ]);
};
