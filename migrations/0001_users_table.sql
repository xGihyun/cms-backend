
CREATE TABLE IF NOT EXISTS users (
id uuid not null default gen_random_uuid(),
created_at timestamp with time zone not null default (
(
    (now())::timestamp without time zone at time zone 'UTC'::text
) at time zone 'Asia/Shanghai'::text
),
email text not null,
password text not null,
first_name text not null,
last_name text not null
)

