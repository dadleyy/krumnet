select u.id, u.name, u.default_email from users as u where u.id = $1 limit 1;
