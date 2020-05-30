select
  id            as user_id,
  name          as user_name,
  default_email as user_email
from
  krumnet.users as users
where
  users.id = $1
limit 1
