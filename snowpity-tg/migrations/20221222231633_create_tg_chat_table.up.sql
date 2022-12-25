create table if not exists tg_chat(
    id bigint not null,
    kind smallint not null,
    title text,
    name text,
    invite_link text,

    updated_at timestamp with time zone not null default (now()::timestamp),
    registered_at timestamp with time zone not null default (now()::timestamp),

    registered_by_user_id bigint not null,
    registered_by_user_name text,
    registered_by_user_full_name text not null,
    registered_by_action smallint not null,

    is_captcha_enabled boolean not null default false,

    constraint tg_chat_pk primary key (id)
);
