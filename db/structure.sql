/* This file will create the database schema from scratch, deleting any existing data.
 */
create extension if not exists "uuid-ossp";

drop schema if exists krumnet cascade;

create schema krumnet;

drop table if exists krumnet.users cascade;

create table krumnet.users (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  default_email varchar unique not null,
  name varchar not null,
  created_at timestamp with time zone default now()
);

drop table if exists krumnet.google_accounts cascade;

create table krumnet.google_accounts (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  email varchar unique not null,
  name varchar not null,
  google_id varchar unique not null,
  user_id varchar references krumnet.users(id) not null
);

drop table if exists krumnet.lobbies cascade;

create table krumnet.lobbies (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  job_id varchar not null,
  name varchar not null,
  settings bit(10) not null, /*
  * | bit  | significance   |
  * | ---- | -------------  |
  * | 1    | public/private |
  */
  created_at timestamp with time zone default now()
);

drop table if exists krumnet.lobby_memberships cascade;

create table krumnet.lobby_memberships (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  user_id varchar references krumnet.users(id) not null,
  lobby_id varchar references krumnet.lobbies(id) not null,
  invited_by varchar references krumnet.users(id),
  permissions bit(10) not null,
  joined_at timestamp with time zone,
  left_at timestamp with time zone
);

drop table if exists krumnet.games cascade;

create table krumnet.games (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  job_id varchar not null,
  lobby_id varchar references krumnet.lobbies(id) not null,
  created_at timestamp with time zone default now()
);

drop table if exists krumnet.game_memberships cascade;

create table krumnet.game_memberships (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  user_id varchar references krumnet.users(id) not null,
  game_id varchar references krumnet.games(id) not null,
  permissions bit(10) not null,
  created_at timestamp with time zone default now()
);

drop table if exists krumnet.game_rounds cascade;

create table krumnet.game_rounds (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  game_id varchar references krumnet.games(id) not null,
  position int not null,
  created_at timestamp with time zone default now(),
  completed_at timestamp with time zone,
  UNIQUE (game_id, position)
);

drop table if exists krumnet.game_round_entries cascade;

create table krumnet.game_round_entries (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  round_id varchar references krumnet.game_rounds(id) not null,
  member_id varchar references krumnet.game_memberships(id) not null,
  entry varchar,
  created_at timestamp with time zone default now(),
  UNIQUE (round_id, member_id)
);
