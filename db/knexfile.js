const fs = require("fs");
const path = require("path");
const debug = require("debug");
const log = debug("krumnet:knexfile");

require("dotenv").config({ path: path.join(__dirname, '../.env') })

const KEY_MAPPING = {
  dbname: "database",
};

function parsePostgresString(input) {
  return input.split(' ').reduce((acc, part) => {
    const [key, value] = part.split('=');
    return { ...acc, [KEY_MAPPING[key] || key]: value };
  }, {});
}

async function fromConfigFile() {
  const configFile = process.env["KRUMNET_TEST_CONFIG_FILE"] || path.resolve(__dirname, "../krumnet-config.json");
  log("attempting to load '%s'", configFile);
  const configData = await fs.promises.readFile(configFile);
  const config = JSON.parse(configData.toString("utf8"));
  return parsePostgresString(config["record_store"]["postgres_uri"]);
}

module.exports = async function() {
  const connection = process.env['DATABASE_URL'] || await fromConfigFile();
  log("loaded config - '%j'", connection);

  return {
    client: "pg",
    connection,
    migrations: {
      tableName: "knex_migrations"
    },
  };
};
