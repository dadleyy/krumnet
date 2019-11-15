select u.id
from users as u
inner join google_accounts as g
on g.user_id = u.id
where google_id = $1 limit 1;
