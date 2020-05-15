/* This file will create the database schema from scratch, deleting any existing data.
 */
create extension if not exists "uuid-ossp";

drop table if exists google_accounts cascade;

create table google_accounts (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  email varchar unique not null,
  name varchar not null,
  google_id varchar unique not null,
  user_id varchar references users(id) not null
);

drop table if exists users cascade;

create table users (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  default_email varchar unique not null,
  name varchar not null,
  created_at timestamp default now()
);

drop table if exists lobbies cascade;

create table lobbies (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  job_id varchar not null,
  name varchar not null,
  settings bit(10) not null, /*
  * | bit  | significance   |
  * | ---- | -------------  |
  * | 1    | public/private |
  */
  created_at timestamp default now()
);

drop table if exists lobby_memberships cascade;

create table lobby_memberships (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  user_id varchar references users(id) not null,
  lobby_id varchar references lobbies(id) not null,
  permissions bit(10) not null,
  joined_at timestamp default now(),
  left_at timestamp
);

drop table if exists games cascade;

create table games (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  lobby_id varchar references lobbies(id) not null,
  created_at timestamp default now()
);

drop table if exists game_memberships cascade;

create table game_memberships (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  user_id varchar references users(id) not null,
  game_id varchar references games(id) not null,
  permissions bit(10) not null,
  created_at timestamp default now()
);

