use poise::serenity_prelude::*;
use regex::Regex;
use std::{str::FromStr};

use crate::{utils::*, Data};
use crate::ticket::Ticket;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;


#[poise::command(slash_command, subcommands("open", "close", "show", "list", "listall"))]
pub async fn ticket(ctx: Context<'_>) -> Result<(), Error> {
    respond(&ctx, String::from("Manage tickets"), true).await;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn open(
    ctx: Context<'_>,
    #[description = "Title"] title: String,
    #[description = "Description"] description: String,
    #[description = "Related users (@mentions seperated by space)"] related_users_option: Option<String>
) -> Result<(), Error> {
    let author: UserId = ctx.author().id;
    let title: String = title.trim_matches('"').to_string();
    let description: String = description.trim_matches('"').to_string();
    let related_users_option: String = related_users_option.unwrap_or_default().trim_matches('"').to_string();
    let related_users: Vec<&str> = related_users_option.split_whitespace().collect();

    match ctx.data().dbms.insert_ticket(&author.to_string(), &title, &description).await {
        Ok(id) => {
            let ticket: Ticket = Ticket {
                id,
                author: author.to_string(),
                title: title.clone(),
                description: description.clone(),
                is_open: true,
            };

            let guild_id: GuildId = ctx.guild_id().unwrap();
            let category_id: ChannelId = ChannelId::from_str(&get_env_var("OPEN_CATEGORY_ID").await)?;
            let channel_name: String = format!("(#{}): {}", ticket.id, ticket.title);
            let mut allowed_users: Vec<UserId> = vec![author];

            let user_id_regex: &str = r"<@!?(\d{17,19})>";
            let regex: Regex = Regex::new(user_id_regex)?;

            for user in related_users {
                if regex.is_match(user) {
                    let user_id_str: &str = user.trim_matches('<').trim_matches('>').trim_start_matches('@');
                    let user_id: UserId = match UserId::from_str(user_id_str) {
                        Ok(id) => id,
                        Err(e) => {
                            eprintln!("{}", e);
                            return Ok(())
                        }
                    };
                    allowed_users.push(user_id);
                }
            }

            let new_channel: ChannelId = create_ticket_channel(guild_id, category_id, &channel_name, allowed_users, &ctx).await.unwrap();
            new_channel.say(&ctx, display_ticket(&ticket, None).await).await.unwrap();

            respond(&ctx, format!("Opened ticket (#{}): {}.", id, title), true).await;
        }
        Err(_) => {
            respond(&ctx, format!("Failed to open ticket {}.", title), true).await;
        }
    }

    Ok(())
}

#[poise::command(slash_command)]
pub async fn close(ctx: Context<'_>, #[description = "Ticket ID"] id: u32) -> Result<(), Error> {
    match ctx.data().dbms.close_ticket(id).await {
        Ok(result) => {
            if result {
                let guild_id: GuildId = ctx.guild_id().unwrap();
                let tickets: Vec<Ticket> = ctx.data().dbms.get_tickets(false).await.expect("Failed to get tickets");
                let closed_category_id: ChannelId = ChannelId::from_str(&get_env_var("CLOSED_CATEGORY_ID").await).unwrap();
                let ticket: &Ticket = tickets.iter().find(|t| t.id == id).expect(&format!("Ticket #{} not found.", id));
                let ticket_channel_title: String = format!("{}-{}", ticket.id, ticket.title);
                let channel_id: ChannelId = get_channel_from_name(guild_id, &ctx, &ticket_channel_title).await.expect("Failed to find channel");

                let _ = match close_ticket_channel(guild_id, channel_id, closed_category_id, &ctx).await {
                    Ok(_) => {
                        respond(&ctx, format!("Closed ticket #{}.", id), true).await;
                    },
                    Err(_) => {
                        respond(&ctx, format!("Failed to close ticket #{}.", id), true).await;
                    }
                };

            } else {
                respond(&ctx, format!("Invalid ticket ID."), true).await;
            }
        }
        Err(_) => {
            respond(&ctx, format!("Failed to close ticket #{}.", id), true).await;
        }
    }

    Ok(())
}

#[poise::command(slash_command)]
pub async fn show(ctx: Context<'_>, #[description = "Ticket ID"] id: u32) -> Result<(), Error> {
    let tickets: Vec<Ticket> = ctx.data().dbms.get_tickets(false).await.expect("Failed to get tickets");

    if tickets.iter().any(|t| t.id == id) {
        for ticket in tickets {
            let author_id: UserId = UserId::new(ticket.author.parse::<u64>().unwrap());

            if ticket.id == id && ctx.author().id == author_id {
                respond(&ctx, display_ticket(&ticket, None).await, true).await;
            
            } else {
                respond(&ctx, format!("This is not your ticket."), true).await;
            }
        }
    } else {
        respond(&ctx, format!("Invalid ticket ID."), true).await;
    }

    Ok(())
}

#[poise::command(slash_command)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let tickets: Vec<Ticket> = ctx.data().dbms.get_tickets(true).await.expect("Failed to get tickets");
    let mut reply: String = String::new();
    
    for ticket in tickets {
        reply = format!("{}\n(#{}): {}", reply, ticket.id, ticket.title);
    }

    if reply.is_empty() {
        reply = "No open tickets.".to_string();
    }

    respond(&ctx, reply, true).await;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn listall(ctx: Context<'_>) -> Result<(), Error> {
    let tickets: Vec<Ticket> = ctx.data().dbms.get_tickets(false).await.expect("Failed to get tickets");
    let mut reply: String = String::new();

    for ticket in tickets {
        let is_open_emoji: &str = if ticket.is_open { ":white_check_mark:" } else { ":x:" };
        reply = format!("{}\n(#{}): {} {}", reply, ticket.id, ticket.title, is_open_emoji);
    }

    if reply.is_empty() {
        reply = "No tickets found.".to_string();
    }

    respond(&ctx, reply, true).await;
    Ok(())
}