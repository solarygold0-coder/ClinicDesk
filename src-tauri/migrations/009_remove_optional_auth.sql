DROP TABLE IF EXISTS auth_sessions;
DROP TABLE IF EXISTS user_roles;
DROP TABLE IF EXISTS role_capabilities;
DROP TABLE IF EXISTS roles;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS security_settings;

DELETE FROM app_meta WHERE key='auth_architecture';
UPDATE app_meta SET value='9' WHERE key='schema_version';
