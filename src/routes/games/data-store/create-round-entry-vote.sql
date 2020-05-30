insert into
  krumnet.game_round_entry_votes
  (round_id, lobby_id, game_id, member_id, user_id, entry_id)
select
  entries.round_id, entries.lobby_id, entries.game_id, members.id, members.user_id, entries.id
from
  krumnet.game_round_entries as entries,
  krumnet.game_memberships as members
where
  entries.id = $1
and
  members.id = $2
returning
  id;
