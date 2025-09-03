use serenity::{
    async_trait,
    framework::standard::macros::group,
    model::{
        application::{command::CommandOptionType,
        interaction::{application_command::ApplicationCommandInteraction, Interaction, InteractionResponseType}}, gateway::Ready, guild::Member, id::{GuildId, RoleId, UserId}
    },
    prelude::*,
};
use tokio_postgres::{Error};
use dotenvy::dotenv;
use std::{env, u32};

use crate::dbms::DBMS;
use crate::ticket::Ticket;

#[group]
struct General;

pub struct Handler {
    pub dbms: DBMS,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        dotenv().ok();
        let guild_id = GuildId(
            env::var("TEST_GUILD_ID")
                .expect("Couldn't find TEST_GUILD_ID environment variable.")
                .parse::<u64>()
                .expect("TEST_GUILD_ID must be a u64"),
        );

        let _ = guild_id.create_application_command(&ctx.http, |cmd| {
            cmd.name("open")
                .description("Open a ticket")
                .create_option(|o| {
                    o.name("title")
                    .description("Ticket name")
                    .kind(CommandOptionType::String)
                    .required(true)
                })
                .create_option(|o| {
                    o.name("description")
                    .description("Ticket description")
                    .kind(CommandOptionType::String)
                    .required(true)
                })
        }).await;

        let _ = guild_id.create_application_command(&ctx.http, |cmd| {
            cmd.name("close")
                .description("Close a ticket")
                .create_option(|o| {
                    o.name("id")
                    .description("Ticket ID")
                    .kind(CommandOptionType::Integer)
                    .required(true)
                })
        }).await;

        let _ = guild_id.create_application_command(&ctx.http, |cmd| {
            cmd.name("show")
            .description("Show a ticket")
            .create_option(|o| {
                o.name("id")
                .description("Ticket ID. Leave empty to show all open tickets.")
                .kind(CommandOptionType::Integer)
                .required(false)
            })
        }).await;

        let _ = guild_id.create_application_command(&ctx.http, |cmd| {
            cmd.name("SHOWALL")
            .description("[MODS ONLY] SHOW ALL TICKETS")
        }).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            self.handle_application_command(&ctx, &command).await;
        }
    }
}

impl Handler {
    pub async fn handle_application_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) {
        let mut reply: String = String::new();
        match command.data.name.as_str() {
            "open" => {
                let title: String = self.get_option(&command, "title").await;
                let description: String = self.get_option(&command, "description").await;

                match self.dbms.insert_ticket(&command.user.id.to_string(), &title, &description).await {
                    Ok(id) => {
                        println!("Opened ticket {}. (#{})", title, id);
                        reply = format!("Opened ticket (#{})\n```{}\n\nDescription:\n{}```", id, title, description);
                    },
                    Err(e) => {
                        println!("Failed to open ticket {}.\nError: {}", title, e);
                        reply = format!("Failed to open ticket {}.", title);
                    }
                }
            }

            "close" => {
                if Self::is_mod(&ctx, &command).await {
                    let id: u32 = self.get_option(&command, "id")
                    .await.parse::<u32>()
                    .unwrap_or(u32::max_value());

                    match self.dbms.close_ticket(id).await {
                        Ok(_) => {
                            reply = format!("Closed ticket #{}.", id);
                        },
                        Err(e) => {
                            println!("Failed to close ticket #{}.\nError: {}", id, e);
                        }
                    }
                } else {
                    reply = format!("Only mods can close tickets.\nThis incident will be reported.");
                }

                println!("{}", reply);
            }

            "show" => {
                let id: u32 = self.get_option(&command, "id")
                .await.parse::<u32>()
                .unwrap_or(u32::max_value());

                let tickets: Vec<Ticket> = self.dbms.get_open_tickets().await.expect("Failed to get tickets.");

                let a = tickets.iter().find(|ticket| ticket.id == id).and_then(|ticket| Result<ticket.id, Error>);

                for ticket in tickets {
                    if ticket.id == id {
                        reply = Self::display_ticket(&ticket).await;
                    }
                }
            }

            "SHOWALL" => {
                if Self::is_mod(&ctx, &command).await {
                    let tickets: Vec<Ticket> = self.dbms.get_open_tickets().await.expect("Failed to get tickets.");

                    for ticket in tickets {
                        reply = format!("{}\n{}", reply, Self::display_ticket(&ticket).await);
                    }
                }
            }

            _ => {
                self.respond(&command, &ctx, "Unknown command.").await;
            }
        }

        if reply != "" {
            self.respond(command, ctx, &reply).await;
        }
    }

    async fn respond(&self, command: &ApplicationCommandInteraction, ctx: &Context, reply: &str) {
        let _ = command.create_interaction_response(&ctx.http, |r| {
            r.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| m.content(reply))
        }).await;
    }

    async fn get_option(&self, command: &ApplicationCommandInteraction, option: &str) -> String {
        let val = command.data.options.iter()
                .find(|opt| opt.name == option)
                .and_then(|opt| opt.value.as_ref());

        match val {
            Some(s) => s.clone().to_string(),
            None => String::from("")
        }
    }

    async fn has_role(member: &Member, role_id: RoleId) -> bool {
        member.roles.contains(&role_id)
    }

    async fn is_mod(ctx: &Context, command: &ApplicationCommandInteraction) -> bool {
        let mod_role_id: RoleId = RoleId(env::var("TEST_MOD_ROLE_ID")
        .expect("Couldn't find TEST_MOD_ROLE_ID environment variable.")
        .parse::<u64>()
        .unwrap());

        let guild_id: GuildId = command.guild_id.unwrap();
        let user_id: UserId = command.user.id;
        let member: Member = guild_id.member(&ctx.http, user_id).await.unwrap();


        Self::has_role(&member, mod_role_id).await
    }

    async fn display_ticket(ticket: &Ticket) -> String {
        let is_open_emoji= if ticket.is_open {
            ":white_check_mark:"
        } else {
            ":red_cross:"
        };

        format!("### (#{}): __{}__\nAuthor: {}\n\nDescription:\n{}\n\nopen: {}", ticket.id, ticket.title, ticket.author, ticket.description, is_open_emoji)
    }
}