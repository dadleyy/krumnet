## Krumnet

Application.

### Local Setup: `krumnet-config.json` & `.env`

The `krumnet` (web process) and `kruwk` (background worker process) require [postgres](https://www.postgresql.org/)
and [redis](https://redis.io/) as backends during runtime. The connection parameters for these, google oauth
credentials and [krumi](https://github.com/krumpled/krumi) environment information are read by the executables from
a file (`krumnet-config.json` by default) during startup, with a fallback for the postgres connection url provided
from the `DATABASE_URL` environment variable, which is _required_ for development (this application uses [`sqlx`] for
query execution).

An example - [`krumnet-config.example.json`](/krumnet-config.example.json) - is available at the root of this
repository and can be copied to `krumnet-config.json`, where it will be ignored by git:

```
$ cp krumnet-config.example.json krumnet-config.json
```

When running the web api, the `google` configuration will need to be populated with values from the [google cloud
console](https://console.cloud.google.com/)'s credentials page.

The schema of this configuration maps directly to the [`Configuration`](/src/configuration.rs#L12-L31) struct - the
file's contents are piped right through [`serde_json::from_slice`](https://docs.serde.rs/serde_json/fn.from_slice.html).

#### Local Setup: Redis

Redis is used as both a background job storage queue as well as the web api's session store. For local development,
either run the redis on your machine's metal, or via [docker](https://www.docker.com/):

```
$ docker run --restart always --name $REDIS_CONTAINER_NAME -p 6379:6379 -d redis
```

#### Local Setup: Postgres

The database schema is managed by [knex](http://knexjs.org/), with it's cli wrapped by a few npm commands in the `db`
directory:

```
$ cd db
$ npm i
$ npm run migrate:up
```

[`sqlx`]: https://docs.rs/sqlx/0.3.5/sqlx/index.html
