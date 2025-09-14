use log::{error, info, warn};
use poise::serenity_prelude::*;
use regex::Regex;
use std::{str::FromStr};

use crate::utils::{*, Context, Error};
use crate::ticket::Ticket;


#[poise::command(
    slash_command,
    name_localized("en-US", "ticket"),
    description_localized("en-US", "Manage tickets"),
    subcommands("open", "close", "show", "list", "listall"),
    subcommand_required
    )]
pub async fn ticket(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(
    slash_command,
    name_localized("en-US", "open"),
    description_localized("en-US", "Open a ticket"),
    hide_in_help
    )]
pub async fn open(
    ctx: Context<'_>,
    #[description = "Title of the ticket"] title: String,
    #[description = "Short description of the ticket"] description: String,
    #[description = "(Optional) Related users (@mentions seperated by space)"] related_users_option: Option<String>
) -> Result<(), Error> {
    let author: UserId = ctx.author().id;
    let title: String = title.trim_matches('"').to_string();
    let description: String = description.trim_matches('"').to_string();
    let related_users_option: String = related_users_option.unwrap_or_default().trim_matches('"').to_string();
    let related_users: Vec<&str> = related_users_option.split_whitespace().collect();

    match ctx.data().dbms.insert_ticket(author.get(), &title, &description).await {
        Ok(id) => {
            let author_u64: u64 = author.get();
            let mut ticket: Ticket = Ticket {
                id,
                author: author_u64,
                title: title.clone(),
                description: description.clone(),
                is_open: true,
                channel_id: 0
            };

            let guild_id: GuildId = ctx.guild_id().unwrap();
            let category_id: ChannelId = ChannelId::from_str(&get_env_var("OPEN_CATEGORY_ID").await)?;
            let mod_role_id: RoleId = RoleId::from_str(&get_env_var("MOD_ROLE_ID").await)?;
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
                            error!("{}", e);
                            return Ok(())
                        }
                    };
                    allowed_users.push(user_id);
                }
            }

            let new_channel: ChannelId = create_ticket_channel(&ctx, guild_id, category_id, &channel_name, allowed_users).await.unwrap();
            ticket.channel_id = new_channel.get();
            ctx.data().dbms.set_channel_id(ticket.id as i32, new_channel.get().to_string()).await.expect(&format!("Failed to set channel_id for ticket {}", ticket.id));
            new_channel.say(&ctx, format!("{}\n<@&{}>", display_ticket(&ticket, None).await, mod_role_id)).await.unwrap();

            info!("{} opened ticket (#{}): {}.", author.to_string(), id, title);
            respond(&ctx, format!("Opened ticket <#{}>", new_channel.to_string()), true).await;
        }
        Err(e) => {
            error!("Failed to open ticket {}: {}", title, e);
            respond(&ctx, format!("Failed to open ticket {}.", title), true).await;
        }
    }

    Ok(())
}

#[poise::command(
    slash_command,
    name_localized("en-US", "close"),
    description_localized("en-US", "Close a ticket"),
    hide_in_help,
    check = "is_mod"
    )]
pub async fn close(
    ctx: Context<'_>,
    #[description = "Ticket ID"] id: u64
) -> Result<(), Error> {
    match ctx.data().dbms.close_ticket(id).await {
        Ok(result) => {
            if result {
                let author: UserId = ctx.author().id;
                let guild_id: GuildId = ctx.guild_id().unwrap();
                let tickets: Vec<Ticket> = ctx.data().dbms.get_tickets(false).await.expect("Failed to get tickets");
                let closed_category_id: ChannelId = ChannelId::from_str(&get_env_var("CLOSED_CATEGORY_ID").await).unwrap();
                let ticket: &Ticket = tickets.iter().find(|t| t.id == id).expect(&format!("Ticket #{} not found.", id));
                let channel_id: ChannelId = ChannelId::new(ctx.data().dbms.get_channel_id(ticket.id as i32).await.unwrap());

                let _ = match close_ticket_channel(guild_id, channel_id, closed_category_id, &ctx).await {
                    Ok(_) => {
                        info!("{} closed ticket #{}.", author.to_string(), id);
                        respond(&ctx, format!("Closed ticket #{}.", id), true).await;
                    },
                    Err(_) => {
                        error!("Failed to close ticket #{}", id);
                        respond(&ctx, format!("Failed to close ticket #{}.", id), true).await;
                    }
                };

            } else {
                warn!("close: Invalid ticket ID {}", id);
                respond(&ctx, format!("Invalid ticket ID."), true).await;
            }
        }
        Err(_) => {
            error!("Failed to close ticket #{}", id);
            respond(&ctx, format!("Failed to close ticket #{}.", id), true).await;
        }
    }

    Ok(())
}

#[poise::command(
    slash_command,
    name_localized("en-US", "show"),
    description_localized("en-US", "Show a ticket"),
    hide_in_help
    )]
pub async fn show(
    ctx: Context<'_>,
    #[description = "Ticket ID"] id: u64
) -> Result<(), Error> {
    let tickets: Vec<Ticket> = ctx.data().dbms.get_tickets(false).await.expect("Failed to get tickets");

    if tickets.iter().any(|t| t.id == id) {
        for ticket in tickets {
            let author_id: UserId = UserId::new(ticket.author as u64);

            if ticket.id == id && ctx.author().id == author_id {
                respond(&ctx, display_ticket(&ticket, None).await, true).await;
            
            }
        }
    } else {
        warn!("show: Invalid ticket ID {}", id);
        respond(&ctx, format!("Invalid ticket ID."), true).await;
    }

    Ok(())
}

#[poise::command(
    slash_command,
    name_localized("en-US", "list"),
    description_localized("en-US", "List all open tickets"),
    hide_in_help,
    check = "is_mod"
    )]
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

#[poise::command(
    slash_command,
    name_localized("en-US", "listall"),
    description_localized("en-US", "List all tickets"),
    hide_in_help,
    check = "is_mod"
    )]
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