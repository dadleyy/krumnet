select
  u.id            as id,
  u.name          as name,
  u.default_email as email
from
  krumnet.users as u
where
  id = $1;
