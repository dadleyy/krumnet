/* Find a user based on the google id returned. */
select u.id
from krumnet.users as u
inner join krumnet.google_accounts as g
on g.user_id = u.id
where g.google_id = $1 limit 1;
