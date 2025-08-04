-- Add up migration script here
create table games (
    id integer primary key autoincrement,
    last_event_index integer not null,
    data string not null
);

create table game_players (
    game_id integer not null,
    player_number integer not null,
    data string not null,
    foreign key(game_id) references games(id)
);
create table game_events (
    game_id integer not null,
    idx integer not null,
    data string not null,
    foreign key(game_id) references games(id)
);
