#!/bin/bash

docker ps -a
docker stop krumnet-redis
docker rm krumnet-redis
docker run --restart always --name krumnet-redis -p 6379:6379 -d redis
