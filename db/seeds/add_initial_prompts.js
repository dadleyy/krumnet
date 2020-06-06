const fs = require("fs");
const path = require("path");
const debug = require("debug");
const log = debug("krumnet:seeds.initial-prompts");

const SOURCE = 'initial-import';

exports.seed = async function(knex) {
  const filename = path.resolve(__dirname, "../migrations/data/2020-05-21-add-prompts.sql");
  log("loading initial prompt seed from '%s'", filename);
  const buffer = await fs.promises.readFile(filename);
  log("clearing existing data from '%s'", SOURCE);
  await knex('krumnet.prompts').where('source', SOURCE).del();
  return knex.raw(buffer.toString("utf-8"));
};
