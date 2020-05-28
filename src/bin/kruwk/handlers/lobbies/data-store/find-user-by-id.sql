select
  u.id, u.name, u.default_email
from
  krumnet.users as u
where
  id = $1;
