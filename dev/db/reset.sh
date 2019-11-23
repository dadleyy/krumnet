#!/bin/bash

psql -f ./db/structure.sql --username postgres --port 8082 krumnet
