use std::env;
use dotenvy::dotenv;
use poise::{serenity_prelude::*, CreateReply};

use crate::{ticket::Ticket, Data};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;


pub async fn create_ticket_channel(ctx: &Context<'_>, guild_id: GuildId, category_id: ChannelId, channel_name: &str, allowed_users: Vec<UserId>) -> Result<ChannelId, Box<dyn std::error::Error>> {
    dotenv().ok();

    let mod_role_id: u64 = get_env_var("MOD_ROLE_ID")
        .await
        .parse::<u64>()
        .expect("MOD_ROLE_ID must be u64");

    let mut overwrites: Vec<PermissionOverwrite> = Vec::new();

    for user in allowed_users {
        overwrites.push(PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(user)
        })
    }

    overwrites.push(PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Role(RoleId::new(mod_role_id))
    });

    overwrites.push(PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::VIEW_CHANNEL,
        kind: PermissionOverwriteType::Role(RoleId::new(guild_id.into()))
    });

    let builder: CreateChannel = CreateChannel::new(channel_name)
        .name(channel_name)
        .kind(ChannelType::Text)
        .category(category_id)
        .permissions(overwrites);

    let channel: GuildChannel = guild_id.create_channel(&ctx.http(), builder).await.unwrap();

    Ok(channel.id)
}

pub async fn close_ticket_channel(guild_id: GuildId, channel_id: ChannelId, closed_category_id: ChannelId, ctx: &Context<'_>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();

    let mut overwrites: Vec<PermissionOverwrite> = Vec::new();
    let mod_role_id: RoleId = RoleId::new(get_env_var("MOD_ROLE_ID").await.parse::<u64>().unwrap());

    let builder: EditChannel<'_> = EditChannel::new().permissions(Vec::new());

    channel_id
        .edit(&ctx.http(), builder)
        .await?;

    overwrites.push(PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::VIEW_CHANNEL,
        kind: PermissionOverwriteType::Role(RoleId::new(guild_id.into())),
    });

    overwrites.push(PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Role(mod_role_id),
    });

    let builder = EditChannel::new().category(closed_category_id).permissions(overwrites);

    channel_id.edit(&ctx.http(), builder).await?;

    Ok(())
}

pub async fn get_env_var(var: &str) -> String {
    env::var(&var).expect(
        &format!("Couldn't find {} environment variable", var)
    )
}

pub async fn respond(ctx: &Context<'_>, reply: String, ephemeral: bool) {
    ctx.send(
        CreateReply::default()
                .content(reply)
                .ephemeral(ephemeral),
        )
        .await.unwrap();
}

pub async fn has_role(member: &Member, role_id: RoleId) -> bool {
    member.roles.contains(&role_id)
}

pub async fn is_mod(ctx: Context<'_>) -> Result<bool, Error> {
    let mod_role_id: RoleId = RoleId::new(
        get_env_var("MOD_ROLE_ID")
            .await
            .parse::<u64>()
            .expect("MOD_ROLE_ID must be u64")
    );

    let member: Member = ctx.author_member().await.unwrap().into_owned();

    Ok(has_role(&member, mod_role_id).await)
}

pub async fn display_ticket(ticket: &Ticket, related_users: Option<Vec<UserId>>) -> String {
    let is_open_emoji: String = String::from(if ticket.is_open {
        ":white_check_mark:"
    } else {
        ":x:"
    });

    let mut related_user_string: String = String::new();

    let related_users = match related_users {
        Some(mut s) => s.split_off(1),
        None => Vec::new(),
    };

    if related_users.iter().len() > 0 {
        related_user_string = String::from("\nRelated Users:");
    }

    for user in related_users {
        related_user_string = format!("{} <@{}>", related_user_string, user.get());
    }

    format!("### (#{}): __{}__\nAuthor: <@{}>{}\n\nDescription:\n{}\n\nopen: {}", ticket.id, ticket.title, ticket.author, related_user_string, ticket.description, is_open_emoji)
}