const fs = require("fs");
const path = require("path");
const debug = require("debug");
const log = debug("krumnet:knexfile");

const KEY_MAPPING = {
  dbname: "database",
};

function parsePostgresString(input) {
  return input.split(' ').reduce((acc, part) => {
    const [key, value] = part.split('=');
    return { ...acc, [KEY_MAPPING[key] || key]: value };
  }, {});
}

module.exports = async function() {
  const configFile = process.env["KRUMNET_TEST_CONFIG_FILE"] || path.resolve(__dirname, "../krumnet-config.json");
  const configData = await fs.promises.readFile("../krumnet-config.json");
  const config = JSON.parse(configData.toString("utf8"));
  const connection = parsePostgresString(config["record_store"]["postgres_uri"]);
  log("loaded config - '%s'", connection);

  return {
    client: "pg",
    connection,
    migrations: {
      tableName: "knex_migrations"
    },
  };
};
