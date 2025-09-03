use serenity::{
    async_trait,
    framework::standard::macros::group,
    model::{
        application::{command::CommandOptionType,
        interaction::{application_command::ApplicationCommandInteraction, Interaction, InteractionResponseType}}, gateway::Ready, guild::Member, id::{GuildId, RoleId, UserId}
    },
    prelude::*,
};
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
        let guild_id: GuildId = GuildId(
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
                .description("Ticket ID.")
                .kind(CommandOptionType::Integer)
                .required(true)
            })
        }).await;

        let _ = guild_id.create_application_command(&ctx.http, |cmd| {
            cmd.name("list")
            .description("[MODS ONLY] List open tickets")
        }).await;

        let _ = guild_id.create_application_command(&ctx.http, |cmd| {
            cmd.name("listall")
            .description("[MODS ONLY] List all tickets")
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
                let author: String = command.user.id.to_string();
                let title: String = String::from(self.get_option(&command, "title").await.trim_matches('"'));
                let description: String = String::from(self.get_option(&command, "description").await.trim_matches('"'));

                match self.dbms.insert_ticket(&author, &title, &description).await {
                    Ok(id) => {
                        let ticket: Ticket = Ticket {
                            id: id,
                            author: author,
                            title: title.clone(),
                            description: description,
                            is_open: true
                        };

                        println!("Opened ticket {}. (#{})", title, id);
                        reply = Self::display_ticket(&ticket).await;
                    },
                    Err(e) => {
                        eprintln!("Failed to open ticket {}.\nError: {}", title, e);
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
                            println!("Closed ticket #{}.", id);
                            reply = format!("Closed ticket #{}.", id);
                        },
                        Err(e) => {
                            eprintln!("Failed to close ticket #{}.\nError: {}", id, e);
                            reply = format!("Failed to close ticket #{}.", id);
                        }
                    }
                } else {
                    reply = String::from("Only mods can close tickets.\nThis incident will be reported.");
                }
            }

            "show" => {
                let id: u32 = self.get_option(&command, "id")
                .await
                .parse::<u32>()
                .unwrap();

                let tickets: Vec<Ticket> = self.dbms.get_tickets(false).await.expect("Failed to get tickets.");

                if tickets.iter().any(|t| t.id == id) {
                    for ticket in tickets {
                        if ticket.id == id {
                            reply = Self::display_ticket(&ticket).await;
                        }
                    }
                } else {
                    reply = format!("Invalid ticket ID.");
                }
            }

            "list" => {
                if Self::is_mod(&ctx, &command).await {
                    let tickets: Vec<Ticket> = self.dbms.get_tickets(true).await.expect("Failed to get tickets.");

                    for ticket in tickets {
                        reply = format!("{}\n(#{}): {}", reply, ticket.id, ticket.title);
                    }
                } else {
                    reply = String::from("Only mods can list open tickets.\nThis incident will be reported.");
                }
            }

            "listall" => {
                if Self::is_mod(&ctx, &command).await {
                    let tickets: Vec<Ticket> = self.dbms.get_tickets(false).await.expect("Failed to get tickets.");

                    for ticket in tickets {
                        let is_open_emoji: &str = if ticket.is_open {
                            ":white_check_mark:"
                        } else {
                            ":x:"
                        };

                        reply = format!("{}\n(#{}): {} {}", reply, ticket.id, ticket.title, is_open_emoji);
                    }
                } else {
                    reply = String::from("Only mods can list all tickets.\nThis incident will be reported.");
                }
            }

            _ => {
                self.respond(&command, &ctx, "Unknown command.").await;
            }
        }

        self.respond(command, ctx, if reply != "" {
            &reply
        } else {
            eprintln!("Error: No response was created. Command: ({} {:?})", &command.data.name, &command.data.options);
            "Error :("
        }).await;
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
            Some(s) => s.to_string(),
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
        .expect("TEST_MOD_ROLE_ID must be a u64"));

        let guild_id: GuildId = command.guild_id.unwrap();
        let user_id: UserId = command.user.id;
        let member: Member = guild_id.member(&ctx.http, user_id).await.unwrap();


        Self::has_role(&member, mod_role_id).await
    }

    async fn display_ticket(ticket: &Ticket) -> String {
        let is_open_emoji: String = String::from(if ticket.is_open {
            ":white_check_mark:"
        } else {
            ":x:"
        });

        format!("### (#{}): __{}__\nAuthor: <@{}>\n\nDescription:\n{}\n\nopen: {}", ticket.id, ticket.title, ticket.author, ticket.description, is_open_emoji)
    }
}