select
  u.id            as user_id,
  u.name          as user_name,
  u.default_email as user_email
from
  krumnet.users as u
where
  u.id = $1
limit 1;
