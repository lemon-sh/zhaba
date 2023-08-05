pragma journal_mode = wal;
pragma synchronous = 0;

create table if not exists posts(
    id integer primary key,
    content text not null,
    image text not null,
    ip text not null,
    asn integer,
    mnt text,
    time integer default (strftime('%s','now'))
);
