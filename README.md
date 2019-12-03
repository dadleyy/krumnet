## Krumnet

Application.


#### Local Setup


```
$ docker run --restart always --name $REDIS_CONTAINER_NAME -p 6379:6379 -d redis
$ psql -f ./db/structure.sql --username $POSTGRES_USERNAME --port $POSTGRES_PORT krumnet
```
