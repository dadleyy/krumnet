--
-- PostgreSQL database dump
--

-- Dumped from database version 10.11
-- Dumped by pg_dump version 10.11

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: krumnet; Type: SCHEMA; Schema: -; Owner: postgres
--

CREATE SCHEMA krumnet;


ALTER SCHEMA krumnet OWNER TO postgres;

--
-- Name: plpgsql; Type: EXTENSION; Schema: -; Owner: 
--

CREATE EXTENSION IF NOT EXISTS plpgsql WITH SCHEMA pg_catalog;


--
-- Name: EXTENSION plpgsql; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION plpgsql IS 'PL/pgSQL procedural language';


--
-- Name: pgcrypto; Type: EXTENSION; Schema: -; Owner: 
--

CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;


--
-- Name: EXTENSION pgcrypto; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION pgcrypto IS 'cryptographic functions';


--
-- Name: uuid-ossp; Type: EXTENSION; Schema: -; Owner: 
--

CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA public;


--
-- Name: EXTENSION "uuid-ossp"; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION "uuid-ossp" IS 'generate universally unique identifiers (UUIDs)';


SET default_tablespace = '';

SET default_with_oids = false;

--
-- Name: game_member_placement_results; Type: TABLE; Schema: krumnet; Owner: postgres
--

CREATE TABLE krumnet.game_member_placement_results (
    id character varying(255) DEFAULT public.uuid_generate_v4() NOT NULL,
    user_id character varying(255) NOT NULL,
    lobby_id character varying(255) NOT NULL,
    member_id character varying(255) NOT NULL,
    game_id character varying(255) NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP,
    place integer NOT NULL,
    vote_count integer DEFAULT 0 NOT NULL
);


ALTER TABLE krumnet.game_member_placement_results OWNER TO postgres;

--
-- Name: game_member_round_placement_results; Type: TABLE; Schema: krumnet; Owner: postgres
--

CREATE TABLE krumnet.game_member_round_placement_results (
    id character varying(255) DEFAULT public.uuid_generate_v4() NOT NULL,
    user_id character varying(255) NOT NULL,
    lobby_id character varying(255) NOT NULL,
    member_id character varying(255) NOT NULL,
    game_id character varying(255) NOT NULL,
    round_id character varying(255) NOT NULL,
    created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP,
    place integer NOT NULL,
    vote_count integer DEFAULT 0 NOT NULL
);


ALTER TABLE krumnet.game_member_round_placement_results OWNER TO postgres;

--
-- Name: game_memberships; Type: TABLE; Schema: krumnet; Owner: postgres
--

CREATE TABLE krumnet.game_memberships (
    id character varying(36) DEFAULT public.uuid_generate_v4() NOT NULL,
    user_id character varying(36) NOT NULL,
    lobby_id character varying(36) NOT NULL,
    lobby_member_id character varying(36) NOT NULL,
    game_id character varying(36) NOT NULL,
    created_at timestamp with time zone DEFAULT now(),
    left_at timestamp with time zone
);


ALTER TABLE krumnet.game_memberships OWNER TO postgres;

--
-- Name: game_round_entries; Type: TABLE; Schema: krumnet; Owner: postgres
--

CREATE TABLE krumnet.game_round_entries (
    id character varying(36) DEFAULT public.uuid_generate_v4() NOT NULL,
    round_id character varying(36) NOT NULL,
    member_id character varying(36) NOT NULL,
    user_id character varying(36) NOT NULL,
    game_id character varying(36) NOT NULL,
    lobby_id character varying(36) NOT NULL,
    entry character varying,
    auto boolean DEFAULT false,
    created_at timestamp with time zone DEFAULT now()
);


ALTER TABLE krumnet.game_round_entries OWNER TO postgres;

--
-- Name: game_round_entry_votes; Type: TABLE; Schema: krumnet; Owner: postgres
--

CREATE TABLE krumnet.game_round_entry_votes (
    id character varying(36) DEFAULT public.uuid_generate_v4() NOT NULL,
    round_id character varying(36) NOT NULL,
    lobby_id character varying(36) NOT NULL,
    game_id character varying(36) NOT NULL,
    member_id character varying(36) NOT NULL,
    user_id character varying(36) NOT NULL,
    entry_id character varying(36) NOT NULL,
    created_at timestamp with time zone DEFAULT now()
);


ALTER TABLE krumnet.game_round_entry_votes OWNER TO postgres;

--
-- Name: game_rounds; Type: TABLE; Schema: krumnet; Owner: postgres
--

CREATE TABLE krumnet.game_rounds (
    id character varying(36) DEFAULT public.uuid_generate_v4() NOT NULL,
    lobby_id character varying(36) NOT NULL,
    game_id character varying(36) NOT NULL,
    "position" integer NOT NULL,
    prompt character varying,
    created_at timestamp with time zone DEFAULT now(),
    started_at timestamp with time zone,
    fulfilled_at timestamp with time zone,
    completed_at timestamp with time zone,
    CONSTRAINT completed_after_fulfilled CHECK ((completed_at > fulfilled_at)),
    CONSTRAINT fulfilled_after_started CHECK ((fulfilled_at > started_at)),
    CONSTRAINT started_after_created CHECK ((started_at >= created_at))
);


ALTER TABLE krumnet.game_rounds OWNER TO postgres;

--
-- Name: games; Type: TABLE; Schema: krumnet; Owner: postgres
--

CREATE TABLE krumnet.games (
    id character varying(36) DEFAULT public.uuid_generate_v4() NOT NULL,
    job_id character varying NOT NULL,
    name character varying NOT NULL,
    lobby_id character varying(36) NOT NULL,
    created_at timestamp with time zone DEFAULT now(),
    ended_at timestamp with time zone
);


ALTER TABLE krumnet.games OWNER TO postgres;

--
-- Name: google_accounts; Type: TABLE; Schema: krumnet; Owner: postgres
--

CREATE TABLE krumnet.google_accounts (
    id character varying(36) DEFAULT public.uuid_generate_v4() NOT NULL,
    email character varying NOT NULL,
    name character varying NOT NULL,
    google_id character varying NOT NULL,
    user_id character varying(36) NOT NULL
);


ALTER TABLE krumnet.google_accounts OWNER TO postgres;

--
-- Name: lobbies; Type: TABLE; Schema: krumnet; Owner: postgres
--

CREATE TABLE krumnet.lobbies (
    id character varying(36) DEFAULT public.uuid_generate_v4() NOT NULL,
    job_id character varying NOT NULL,
    name character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now(),
    closed_at timestamp with time zone
);


ALTER TABLE krumnet.lobbies OWNER TO postgres;

--
-- Name: lobby_memberships; Type: TABLE; Schema: krumnet; Owner: postgres
--

CREATE TABLE krumnet.lobby_memberships (
    id character varying(36) DEFAULT public.uuid_generate_v4() NOT NULL,
    user_id character varying(36) NOT NULL,
    lobby_id character varying(36) NOT NULL,
    invited_by character varying(36),
    joined_at timestamp with time zone,
    left_at timestamp with time zone
);


ALTER TABLE krumnet.lobby_memberships OWNER TO postgres;

--
-- Name: prompts; Type: TABLE; Schema: krumnet; Owner: postgres
--

CREATE TABLE krumnet.prompts (
    id character varying(36) DEFAULT public.uuid_generate_v4() NOT NULL,
    number integer NOT NULL,
    prompt character varying NOT NULL,
    source character varying,
    created_by character varying(36),
    approved boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now()
);


ALTER TABLE krumnet.prompts OWNER TO postgres;

--
-- Name: prompts_number_seq; Type: SEQUENCE; Schema: krumnet; Owner: postgres
--

CREATE SEQUENCE krumnet.prompts_number_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER TABLE krumnet.prompts_number_seq OWNER TO postgres;

--
-- Name: prompts_number_seq; Type: SEQUENCE OWNED BY; Schema: krumnet; Owner: postgres
--

ALTER SEQUENCE krumnet.prompts_number_seq OWNED BY krumnet.prompts.number;


--
-- Name: users; Type: TABLE; Schema: krumnet; Owner: postgres
--

CREATE TABLE krumnet.users (
    id character varying(36) DEFAULT public.uuid_generate_v4() NOT NULL,
    default_email character varying NOT NULL,
    name character varying NOT NULL,
    created_at timestamp with time zone DEFAULT now()
);


ALTER TABLE krumnet.users OWNER TO postgres;

--
-- Name: knex_migrations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.knex_migrations (
    id integer NOT NULL,
    name character varying(255),
    batch integer,
    migration_time timestamp with time zone
);


ALTER TABLE public.knex_migrations OWNER TO postgres;

--
-- Name: knex_migrations_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.knex_migrations_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER TABLE public.knex_migrations_id_seq OWNER TO postgres;

--
-- Name: knex_migrations_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.knex_migrations_id_seq OWNED BY public.knex_migrations.id;


--
-- Name: knex_migrations_lock; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.knex_migrations_lock (
    index integer NOT NULL,
    is_locked integer
);


ALTER TABLE public.knex_migrations_lock OWNER TO postgres;

--
-- Name: knex_migrations_lock_index_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.knex_migrations_lock_index_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER TABLE public.knex_migrations_lock_index_seq OWNER TO postgres;

--
-- Name: knex_migrations_lock_index_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.knex_migrations_lock_index_seq OWNED BY public.knex_migrations_lock.index;


--
-- Name: prompts number; Type: DEFAULT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.prompts ALTER COLUMN number SET DEFAULT nextval('krumnet.prompts_number_seq'::regclass);


--
-- Name: knex_migrations id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.knex_migrations ALTER COLUMN id SET DEFAULT nextval('public.knex_migrations_id_seq'::regclass);


--
-- Name: knex_migrations_lock index; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.knex_migrations_lock ALTER COLUMN index SET DEFAULT nextval('public.knex_migrations_lock_index_seq'::regclass);


--
-- Name: game_member_placement_results game_member_placement_results_id_unique; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_placement_results
    ADD CONSTRAINT game_member_placement_results_id_unique UNIQUE (id);


--
-- Name: game_member_placement_results game_member_placement_results_pkey; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_placement_results
    ADD CONSTRAINT game_member_placement_results_pkey PRIMARY KEY (id);


--
-- Name: game_member_round_placement_results game_member_round_placement_results_id_unique; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_round_placement_results
    ADD CONSTRAINT game_member_round_placement_results_id_unique UNIQUE (id);


--
-- Name: game_member_round_placement_results game_member_round_placement_results_pkey; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_round_placement_results
    ADD CONSTRAINT game_member_round_placement_results_pkey PRIMARY KEY (id);


--
-- Name: game_memberships game_memberships_pkey; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_memberships
    ADD CONSTRAINT game_memberships_pkey PRIMARY KEY (id);


--
-- Name: game_round_entries game_round_entries_pkey; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_round_entries
    ADD CONSTRAINT game_round_entries_pkey PRIMARY KEY (id);


--
-- Name: game_round_entries game_round_entries_round_id_member_id_key; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_round_entries
    ADD CONSTRAINT game_round_entries_round_id_member_id_key UNIQUE (round_id, member_id);


--
-- Name: game_round_entry_votes game_round_entry_votes_pkey; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_round_entry_votes
    ADD CONSTRAINT game_round_entry_votes_pkey PRIMARY KEY (id);


--
-- Name: game_round_entry_votes game_round_entry_votes_round_id_member_id_key; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_round_entry_votes
    ADD CONSTRAINT game_round_entry_votes_round_id_member_id_key UNIQUE (round_id, member_id);


--
-- Name: game_rounds game_rounds_game_id_position_key; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_rounds
    ADD CONSTRAINT game_rounds_game_id_position_key UNIQUE (game_id, "position");


--
-- Name: game_rounds game_rounds_pkey; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_rounds
    ADD CONSTRAINT game_rounds_pkey PRIMARY KEY (id);


--
-- Name: games games_name_key; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.games
    ADD CONSTRAINT games_name_key UNIQUE (name);


--
-- Name: games games_pkey; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.games
    ADD CONSTRAINT games_pkey PRIMARY KEY (id);


--
-- Name: google_accounts google_accounts_email_key; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.google_accounts
    ADD CONSTRAINT google_accounts_email_key UNIQUE (email);


--
-- Name: google_accounts google_accounts_google_id_key; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.google_accounts
    ADD CONSTRAINT google_accounts_google_id_key UNIQUE (google_id);


--
-- Name: google_accounts google_accounts_pkey; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.google_accounts
    ADD CONSTRAINT google_accounts_pkey PRIMARY KEY (id);


--
-- Name: lobbies lobbies_name_key; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.lobbies
    ADD CONSTRAINT lobbies_name_key UNIQUE (name);


--
-- Name: lobbies lobbies_pkey; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.lobbies
    ADD CONSTRAINT lobbies_pkey PRIMARY KEY (id);


--
-- Name: lobby_memberships lobby_memberships_pkey; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.lobby_memberships
    ADD CONSTRAINT lobby_memberships_pkey PRIMARY KEY (id);


--
-- Name: prompts prompts_pkey; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.prompts
    ADD CONSTRAINT prompts_pkey PRIMARY KEY (id);


--
-- Name: prompts prompts_prompt_key; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.prompts
    ADD CONSTRAINT prompts_prompt_key UNIQUE (prompt);


--
-- Name: game_member_placement_results single_game_winner; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_placement_results
    ADD CONSTRAINT single_game_winner UNIQUE (place, game_id);


--
-- Name: game_member_placement_results single_member_game_placement; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_placement_results
    ADD CONSTRAINT single_member_game_placement UNIQUE (member_id, game_id);


--
-- Name: game_member_round_placement_results single_member_round_placement; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_round_placement_results
    ADD CONSTRAINT single_member_round_placement UNIQUE (member_id, round_id);


--
-- Name: lobby_memberships single_membership; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.lobby_memberships
    ADD CONSTRAINT single_membership UNIQUE (user_id, lobby_id);


--
-- Name: game_member_round_placement_results single_round_winner; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_round_placement_results
    ADD CONSTRAINT single_round_winner UNIQUE (place, round_id);


--
-- Name: users users_default_email_key; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.users
    ADD CONSTRAINT users_default_email_key UNIQUE (default_email);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: knex_migrations_lock knex_migrations_lock_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.knex_migrations_lock
    ADD CONSTRAINT knex_migrations_lock_pkey PRIMARY KEY (index);


--
-- Name: knex_migrations knex_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.knex_migrations
    ADD CONSTRAINT knex_migrations_pkey PRIMARY KEY (id);


--
-- Name: game_memberships_lobby_id_idx; Type: INDEX; Schema: krumnet; Owner: postgres
--

CREATE INDEX game_memberships_lobby_id_idx ON krumnet.game_memberships USING btree (lobby_id);


--
-- Name: game_round_entries_game_id_idx; Type: INDEX; Schema: krumnet; Owner: postgres
--

CREATE INDEX game_round_entries_game_id_idx ON krumnet.game_round_entries USING btree (game_id);


--
-- Name: game_round_entries_lobby_id_idx; Type: INDEX; Schema: krumnet; Owner: postgres
--

CREATE INDEX game_round_entries_lobby_id_idx ON krumnet.game_round_entries USING btree (lobby_id);


--
-- Name: game_rounds_lobby_id_idx; Type: INDEX; Schema: krumnet; Owner: postgres
--

CREATE INDEX game_rounds_lobby_id_idx ON krumnet.game_rounds USING btree (lobby_id);


--
-- Name: game_member_placement_results game_member_placement_results_game_id_foreign; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_placement_results
    ADD CONSTRAINT game_member_placement_results_game_id_foreign FOREIGN KEY (game_id) REFERENCES krumnet.games(id);


--
-- Name: game_member_placement_results game_member_placement_results_lobby_id_foreign; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_placement_results
    ADD CONSTRAINT game_member_placement_results_lobby_id_foreign FOREIGN KEY (lobby_id) REFERENCES krumnet.lobbies(id);


--
-- Name: game_member_placement_results game_member_placement_results_member_id_foreign; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_placement_results
    ADD CONSTRAINT game_member_placement_results_member_id_foreign FOREIGN KEY (member_id) REFERENCES krumnet.game_memberships(id);


--
-- Name: game_member_placement_results game_member_placement_results_user_id_foreign; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_placement_results
    ADD CONSTRAINT game_member_placement_results_user_id_foreign FOREIGN KEY (user_id) REFERENCES krumnet.users(id);


--
-- Name: game_member_round_placement_results game_member_round_placement_results_game_id_foreign; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_round_placement_results
    ADD CONSTRAINT game_member_round_placement_results_game_id_foreign FOREIGN KEY (game_id) REFERENCES krumnet.games(id);


--
-- Name: game_member_round_placement_results game_member_round_placement_results_lobby_id_foreign; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_round_placement_results
    ADD CONSTRAINT game_member_round_placement_results_lobby_id_foreign FOREIGN KEY (lobby_id) REFERENCES krumnet.lobbies(id);


--
-- Name: game_member_round_placement_results game_member_round_placement_results_member_id_foreign; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_round_placement_results
    ADD CONSTRAINT game_member_round_placement_results_member_id_foreign FOREIGN KEY (member_id) REFERENCES krumnet.game_memberships(id);


--
-- Name: game_member_round_placement_results game_member_round_placement_results_round_id_foreign; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_round_placement_results
    ADD CONSTRAINT game_member_round_placement_results_round_id_foreign FOREIGN KEY (round_id) REFERENCES krumnet.game_rounds(id);


--
-- Name: game_member_round_placement_results game_member_round_placement_results_user_id_foreign; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_member_round_placement_results
    ADD CONSTRAINT game_member_round_placement_results_user_id_foreign FOREIGN KEY (user_id) REFERENCES krumnet.users(id);


--
-- Name: game_memberships game_memberships_game_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_memberships
    ADD CONSTRAINT game_memberships_game_id_fkey FOREIGN KEY (game_id) REFERENCES krumnet.games(id);


--
-- Name: game_memberships game_memberships_lobby_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_memberships
    ADD CONSTRAINT game_memberships_lobby_id_fkey FOREIGN KEY (lobby_id) REFERENCES krumnet.lobbies(id);


--
-- Name: game_memberships game_memberships_lobby_member_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_memberships
    ADD CONSTRAINT game_memberships_lobby_member_id_fkey FOREIGN KEY (lobby_member_id) REFERENCES krumnet.lobby_memberships(id);


--
-- Name: game_memberships game_memberships_user_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_memberships
    ADD CONSTRAINT game_memberships_user_id_fkey FOREIGN KEY (user_id) REFERENCES krumnet.users(id);


--
-- Name: game_round_entries game_round_entries_game_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_round_entries
    ADD CONSTRAINT game_round_entries_game_id_fkey FOREIGN KEY (game_id) REFERENCES krumnet.games(id);


--
-- Name: game_round_entries game_round_entries_lobby_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_round_entries
    ADD CONSTRAINT game_round_entries_lobby_id_fkey FOREIGN KEY (lobby_id) REFERENCES krumnet.lobbies(id);


--
-- Name: game_round_entries game_round_entries_member_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_round_entries
    ADD CONSTRAINT game_round_entries_member_id_fkey FOREIGN KEY (member_id) REFERENCES krumnet.game_memberships(id);


--
-- Name: game_round_entries game_round_entries_round_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_round_entries
    ADD CONSTRAINT game_round_entries_round_id_fkey FOREIGN KEY (round_id) REFERENCES krumnet.game_rounds(id);


--
-- Name: game_round_entries game_round_entries_user_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_round_entries
    ADD CONSTRAINT game_round_entries_user_id_fkey FOREIGN KEY (user_id) REFERENCES krumnet.users(id);


--
-- Name: game_round_entry_votes game_round_entry_votes_entry_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_round_entry_votes
    ADD CONSTRAINT game_round_entry_votes_entry_id_fkey FOREIGN KEY (entry_id) REFERENCES krumnet.game_round_entries(id);


--
-- Name: game_round_entry_votes game_round_entry_votes_game_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_round_entry_votes
    ADD CONSTRAINT game_round_entry_votes_game_id_fkey FOREIGN KEY (game_id) REFERENCES krumnet.games(id);


--
-- Name: game_round_entry_votes game_round_entry_votes_lobby_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_round_entry_votes
    ADD CONSTRAINT game_round_entry_votes_lobby_id_fkey FOREIGN KEY (lobby_id) REFERENCES krumnet.lobbies(id);


--
-- Name: game_round_entry_votes game_round_entry_votes_member_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_round_entry_votes
    ADD CONSTRAINT game_round_entry_votes_member_id_fkey FOREIGN KEY (member_id) REFERENCES krumnet.game_memberships(id);


--
-- Name: game_round_entry_votes game_round_entry_votes_round_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_round_entry_votes
    ADD CONSTRAINT game_round_entry_votes_round_id_fkey FOREIGN KEY (round_id) REFERENCES krumnet.game_rounds(id);


--
-- Name: game_round_entry_votes game_round_entry_votes_user_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_round_entry_votes
    ADD CONSTRAINT game_round_entry_votes_user_id_fkey FOREIGN KEY (user_id) REFERENCES krumnet.users(id);


--
-- Name: game_rounds game_rounds_game_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_rounds
    ADD CONSTRAINT game_rounds_game_id_fkey FOREIGN KEY (game_id) REFERENCES krumnet.games(id);


--
-- Name: game_rounds game_rounds_lobby_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.game_rounds
    ADD CONSTRAINT game_rounds_lobby_id_fkey FOREIGN KEY (lobby_id) REFERENCES krumnet.lobbies(id);


--
-- Name: games games_lobby_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.games
    ADD CONSTRAINT games_lobby_id_fkey FOREIGN KEY (lobby_id) REFERENCES krumnet.lobbies(id);


--
-- Name: google_accounts google_accounts_user_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.google_accounts
    ADD CONSTRAINT google_accounts_user_id_fkey FOREIGN KEY (user_id) REFERENCES krumnet.users(id);


--
-- Name: lobby_memberships lobby_memberships_invited_by_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.lobby_memberships
    ADD CONSTRAINT lobby_memberships_invited_by_fkey FOREIGN KEY (invited_by) REFERENCES krumnet.users(id);


--
-- Name: lobby_memberships lobby_memberships_lobby_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.lobby_memberships
    ADD CONSTRAINT lobby_memberships_lobby_id_fkey FOREIGN KEY (lobby_id) REFERENCES krumnet.lobbies(id);


--
-- Name: lobby_memberships lobby_memberships_user_id_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.lobby_memberships
    ADD CONSTRAINT lobby_memberships_user_id_fkey FOREIGN KEY (user_id) REFERENCES krumnet.users(id);


--
-- Name: prompts prompts_created_by_fkey; Type: FK CONSTRAINT; Schema: krumnet; Owner: postgres
--

ALTER TABLE ONLY krumnet.prompts
    ADD CONSTRAINT prompts_created_by_fkey FOREIGN KEY (created_by) REFERENCES krumnet.users(id);


--
-- PostgreSQL database dump complete
--

