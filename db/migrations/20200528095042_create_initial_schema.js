const debug = require('debug');
const fs = require('fs');
const log = debug('krumnet:migrations.initial-schema');

exports.up = async function(knex) {
  const data = await fs.promises.readFile('./structure.sql');
  return knex.raw(data.toString('utf8'));
};

exports.down = function(knex) {
  log('going down');
  return knex.raw("drop schema krumnet cascade");
};
