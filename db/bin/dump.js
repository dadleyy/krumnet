const { exec } = require("child_process");
const path = require("path");
const fs = require("fs");
const util = require('util');

const debug = require("debug");
const parser = require("pg-connection-string");
const DUMP_FILE = path.resolve(__dirname, '../dump.sql');
const loadConfig = require("../knexfile.js");

require("dotenv").config({ path: path.join(__dirname, '../.env') })

const log = debug("krumnet:db-dump");
log("attempting to find reasonable version of 'pg_dump'");

function findExe() {
  return util.promisify(exec)("which pg_dump")
    .then(({ stdout, stderr }) => {
      log("potentially found path - '%s'", stdout);
      return stdout.toString().trim();
    }).then((potentialPath) => {
      return util.promisify(fs.exists)(potentialPath)
        .then((ok) => {
          if (!ok) {
            return Promise.reject("'%s' did not exist", potentialPath);
          }

          log("'%s' exists, exectuing w/ connection string", potentialPath);
          return potentialPath;
        });
    });
}

Promise.all([
  findExe(),
  loadConfig(),
]).then(([dumpPath, knexConfig]) => {
  const config = parser.parse(knexConfig.connection);
  const options = [
    config.port ? `--port ${config.port}` : null,
    config.database,
  ].filter(Boolean);
  const command = `${dumpPath} --schema-only ${options.join(' ')} > ${DUMP_FILE}`;
  log("executing '%s'", command);
  return util.promisify(exec)(command);
}).then(({ stdout, stderr }) => {
  console.log('Success!');
}).catch((error) => {
  console.error(`Unable to create dump - ${error.message}`);
});

