const fs = require("fs");
const path = require("path");
const debug = require("debug");
const log = debug("krumnet:knexfile");

require("dotenv").config({ path: path.join(__dirname, "../.env") })

const TEST_FILE = path.resolve(__dirname, '../krumnet-config.example.json');
const DEFAULT_FILE = path.resolve(__dirname, "../krumnet-config.json");

const KEY_MAPPING = {
  dbname: "database",
};

function handleFailedFile(error) {
    log("unable to load file '%s'", error);
    return new Buffer('{}');
}

async function fromConfigFile(file) {
  const configData = await fs.promises.readFile(file).catch(handleFailedFile);
  const config = JSON.parse(configData.toString("utf8"));
  return config["record_store"]["postgres_uri"];
}

module.exports = async function() {
  const fromEnv = process.env["DATABASE_URL"];
  const file = process.env["NODE_ENV"] === "test" ? TEST_FILE : DEFAULT_FILE;
  const configUri = await fromConfigFile(file);
  const connection = configUri || fromEnv;

  log("  file: '%s'", configUri);
  log("config: '%s'", fromEnv);
  log(" using: '%s'", connection);

  return {
    client: "pg",
    connection,
    migrations: {
      tableName: "knex_migrations",
    },
  };
};
