use serenity::{
    async_trait,
    framework::standard::macros::group,
    model::{
        application::{command::CommandOptionType,
        interaction::{application_command::ApplicationCommandInteraction, Interaction, InteractionResponseType}},
        gateway::Ready, id::GuildId,
    },
    prelude::*,
};

use serde_json::Value;
use std::{env, result};
use dotenv::dotenv;

use crate::dbms::{self, DBMS};

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
                        .required(false)
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
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            self.handle_application_command(&ctx, &command).await;
        }
    }
}

impl Handler {
    pub async fn handle_application_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) {
        let find = |name: &str| -> Option<&Value> {
            command.data.options.iter()
                .find(|opt| opt.name == name)
                .and_then(|opt| opt.value.as_ref())
        };

        match command.data.name.as_str() {
            "open" => {
                let mut reply = String::new();

                let title = match find("title").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => {
                        self.respond(command, ctx, "Missing ticket title.").await;
                        return;
                    }
                };

                let description = find("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No description provided.");

                let result = self.dbms.insert_ticket(&command.user.id.to_string(), title, description, true).await;
                match result {
                    Ok(id) => {
                        println!("Opened ticket {}. (#{})", title, id);
                        reply = format!("Opened ticket (#{})\n```{}\n\nDescription:\n{}```", id, title, description);
                    },
                    Err(e) => {
                        println!("Failed to open ticket {}.\nError: {}", title, e);
                    }
                }

                self.respond(command, ctx, &reply).await;
            }

            "close" => {
                let mut reply = String::new();

                let id = match find("id").and_then(|v| v.as_i64()) {
                    Some(n) if n >= 0 && n <= u32::MAX as i64 => n as u32,
                    _ => {
                        self.respond(command, ctx, "Invalid ID.").await;
                        return;
                    }
                };

                let result = self.dbms.close_ticket(id).await;

                match result {
                    Ok(_) => {
                        reply = format!("Closed ticket #{}.", id);
                    },
                    Err(e) => {
                        println!("Failed to close ticket #{}.\nError: {}", id, e);
                    }
                }
                self.respond(command, ctx, &reply).await
            }

            _ => {
                let _ = command
                    .create_interaction_response(&ctx.http, |resp| {
                        resp.kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|m| m.content("Unknown command"))
                    })
                    .await;
            }
        }
    }

    pub async fn respond(&self, command: &ApplicationCommandInteraction, ctx: &Context, reply: &str) {
        let _ = command.create_interaction_response(&ctx.http, |r| {
            r.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| m.content(reply))
        }).await;
    }
}