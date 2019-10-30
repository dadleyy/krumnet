create extension if not exists "uuid-ossp";

drop table if exists google_accounts cascade;

create table google_accounts (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  email varchar unique not null,
  name varchar unique not null,
  google_id varchar unique not null,
  user_id varchar references users(id) not null
);

drop table if exists users cascade;

create table users (
  id varchar unique default uuid_generate_v4() PRIMARY KEY,
  default_email varchar unique not null,
  name varchar unique not null
);

