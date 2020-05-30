select
  users.id::text as user_id
from
  krumnet.users as users
inner join
  krumnet.google_accounts as google
on
  google.user_id = users.id
where
  google.google_id = $1
limit 1;
