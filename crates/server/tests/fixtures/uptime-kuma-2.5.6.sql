-- Petite base `kuma.db` de test, au schéma d'Uptime Kuma 2.5.6 (db/knex_init_db.js et
-- db/knex_migrations/ : tables monitor, tag, monitor_tag, notification, heartbeat).
-- Seules les colonnes de `monitor` qui existent dans le dépôt d'Uptime Kuma sont déclarées ;
-- les tests la chargent dans un fichier SQLite temporaire. Les « secrets » ci-dessous ne
-- doivent jamais ressortir de l'import.

CREATE TABLE monitor (
  id integer PRIMARY KEY AUTOINCREMENT NOT NULL,
  name varchar(150),
  active boolean NOT NULL DEFAULT 1,
  user_id integer,
  "interval" integer NOT NULL DEFAULT 20,
  url text,
  type varchar(20),
  weight integer DEFAULT 2000,
  hostname varchar(255),
  port integer,
  created_date datetime NOT NULL DEFAULT CURRENT_TIMESTAMP,
  keyword varchar(255),
  maxretries integer NOT NULL DEFAULT 0,
  ignore_tls boolean NOT NULL DEFAULT 0,
  upside_down boolean NOT NULL DEFAULT 0,
  maxredirects integer NOT NULL DEFAULT 10,
  accepted_statuscodes_json text NOT NULL DEFAULT '["200-299"]',
  dns_resolve_type varchar(5),
  dns_resolve_server varchar(255),
  dns_last_result varchar(255),
  retry_interval integer NOT NULL DEFAULT 0,
  push_token varchar(20) DEFAULT NULL,
  method text NOT NULL DEFAULT 'GET',
  body text DEFAULT NULL,
  headers text DEFAULT NULL,
  basic_auth_user text DEFAULT NULL,
  basic_auth_pass text DEFAULT NULL,
  docker_host integer,
  docker_container varchar(255),
  proxy_id integer,
  expiry_notification boolean DEFAULT 1,
  database_connection_string varchar(2000),
  description text DEFAULT NULL,
  parent integer REFERENCES monitor (id) ON DELETE SET NULL ON UPDATE CASCADE,
  invert_keyword boolean NOT NULL DEFAULT 0,
  json_path text,
  expected_value varchar(255),
  oauth_client_id text DEFAULT NULL,
  oauth_client_secret text DEFAULT NULL,
  timeout double NOT NULL DEFAULT 0
);

CREATE TABLE tag (
  id integer PRIMARY KEY AUTOINCREMENT NOT NULL,
  name varchar(255) NOT NULL,
  color varchar(255) NOT NULL,
  created_date datetime NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE monitor_tag (
  id integer PRIMARY KEY AUTOINCREMENT NOT NULL,
  monitor_id integer NOT NULL REFERENCES monitor (id) ON DELETE CASCADE ON UPDATE CASCADE,
  tag_id integer NOT NULL REFERENCES tag (id) ON DELETE CASCADE ON UPDATE CASCADE,
  value text
);

CREATE TABLE notification (
  id integer PRIMARY KEY AUTOINCREMENT NOT NULL,
  name varchar(255),
  active boolean NOT NULL DEFAULT 1,
  user_id integer,
  is_default boolean NOT NULL DEFAULT 0,
  config longtext
);

-- L'historique des mesures fait l'essentiel du poids d'un vrai fichier : jamais lu.
CREATE TABLE heartbeat (
  id integer PRIMARY KEY AUTOINCREMENT NOT NULL,
  important boolean NOT NULL DEFAULT 0,
  monitor_id integer NOT NULL REFERENCES monitor (id) ON DELETE CASCADE ON UPDATE CASCADE,
  status smallint NOT NULL,
  msg text,
  time datetime NOT NULL,
  ping integer,
  duration integer NOT NULL DEFAULT 0
);

INSERT INTO monitor (id, name, type, parent, active, "interval") VALUES
  (1, 'Clients', 'group', NULL, 1, 60),
  (2, 'Acme', 'group', 1, 1, 60);

INSERT INTO monitor (id, name, type, parent, url, method, "interval", timeout, maxredirects,
                     accepted_statuscodes_json, headers, basic_auth_user, basic_auth_pass)
VALUES (3, 'Blog', 'http', 2, 'https://blog.example.com/', 'GET', 60, 48.0, 10,
        '["200-299","301"]', '{"Authorization": "Bearer abc123"}', 'admin', 'hunter2');

INSERT INTO monitor (id, name, type, url, "interval", keyword, invert_keyword, ignore_tls)
VALUES (4, 'Status page', 'keyword', 'https://status.example.org', 20,
        'All systems operational', 1, 1);

INSERT INTO monitor (id, name, type, url, method, "interval", json_path, expected_value, body)
VALUES (5, 'API health', 'json-query', 'https://api.example.com/health', 'POST', 120,
        '$.status', 'ok', '{"token":"s3cr3t-token"}');

INSERT INTO monitor (id, name, type, hostname, "interval") VALUES
  (6, 'Router', 'ping', '192.168.1.1', 30);

INSERT INTO monitor (id, name, type, hostname, port, "interval") VALUES
  (7, 'NAS SSH', 'port', 'nas.lan', 22, 60),
  (8, 'IPv6 web', 'port', '2001:db8::1', 443, 60);

INSERT INTO monitor (id, name, type, hostname, port, dns_resolve_type, dns_resolve_server,
                     "interval")
VALUES (9, 'MX example', 'dns', 'example.com', 53, 'MX', '1.1.1.1', 300);

INSERT INTO monitor (id, name, type, active, push_token, "interval") VALUES
  (10, 'Nightly backup', 'push', 0, 'pushtok-0123', 86400);

INSERT INTO monitor (id, name, type, docker_container, docker_host, "interval") VALUES
  (11, 'Containers', 'docker', 'web', 1, 60);

INSERT INTO monitor (id, name, type, url, upside_down, "interval") VALUES
  (12, 'Inverted', 'http', 'https://should-be-down.example.com', 1, 60);

INSERT INTO monitor (id, name, type, database_connection_string, "interval") VALUES
  (13, 'Postgres', 'postgres', 'postgres://app:s3cr3t-token@db.lan/app', 60);

INSERT INTO tag (id, name, color) VALUES (1, 'env', '#059669'), (2, 'Critical', '#DC2626');
INSERT INTO monitor_tag (monitor_id, tag_id, value) VALUES (3, 1, 'prod'), (3, 2, '');

INSERT INTO notification (id, name, config) VALUES
  (1, 'Telegram ops', '{"type":"telegram","telegramBotToken":"s3cr3t-token"}'),
  (2, 'Mail', '{"type":"smtp","smtpPassword":"hunter2"}');

INSERT INTO heartbeat (monitor_id, status, time, ping) VALUES (3, 1, '2026-10-01 10:00:00', 42);
