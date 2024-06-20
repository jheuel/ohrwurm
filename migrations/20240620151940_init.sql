-- Add migration script here
CREATE TABLE IF NOT EXISTS tracks
(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    channel TEXT NOT NULL,
    duration TEXT NOT NULL,
    thumbnail TEXT NOT NULL,
    updated DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS queries
(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    guild_id TEXT NOT NULL,
    track_id NUMBER NOT NULL,
    updated DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS users
(
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    global_name TEXT,
    updated DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS guilds
(
    id TEXT PRIMARY KEY,
    updated DATETIME NOT NULL
);
