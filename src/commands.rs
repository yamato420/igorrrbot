use serenity::{
    async_trait,
    framework::standard::{macros::{command, group}, Command, CommandResult},
    model::{
        application::{command::CommandOptionType,
        interaction::{application_command::ApplicationCommandInteraction, Interaction, InteractionResponseType}},
        gateway::Ready, id::GuildId, prelude::*,
    },
    prelude::*,
};

use serde_json::Value;
use std::env;
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
            cmd.name("ticket")
                .description("Create or manage a ticket")
                .create_option(|o| {
                    o.name("option")
                        .description("Command option")
                        .kind(CommandOptionType::String)
                        .required(true)
                })
                .create_option(|o| {
                    o.name("name")
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
        })
        .await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            self.handle_application_command(&ctx, &command).await;
        }
    }
}

impl Handler {
    pub async fn handle_application_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) {
        match command.data.name.as_str() {
            "ticket" => {
                let find = |name: &str| -> Option<&Value> {
                    command.data.options.iter()
                        .find(|opt| opt.name == name)
                        .and_then(|opt| opt.value.as_ref())
                };

                let option = match find("option").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => {
                        let _ = command.create_interaction_response(&ctx.http, |r| {
                            r.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|m| m.content("Missing option."))
                        }).await;
                        return;
                    }
                };

                let title = match find("name").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => {
                        let _ = command.create_interaction_response(&ctx.http, |r| {
                            r.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|m| m.content("Missing ticket name."))
                        }).await;
                        return;
                    }
                };

                let description = find("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No description provided.");


                let mut reply = String::new();

                match option {
                    "create" => {
                        let result = self.dbms.insert_ticket(&command.user.id.to_string(), title, description, true).await;

                        match result {
                            Ok(_) => {
                                println!("Created ticket {}.", title);
                            },
                            Err(e) => {
                                println!("Failed to create ticket {}.", title);
                            }
                        }

                        reply = format!("Created Ticket\n```{}\n\nDescription:\n{}```", title, description);
                    },

                    "view" => {
                        reply = format!("```{}\n\nDescription:\n{}```", title, description); // TODO: take ID as param, search ticket in DB, display ticket
                    },

                    "close" => {
                        reply = format!("# {}\n{}", title, description); // TODO: set ticket is_open to false
                    },

                    _ => {
                        reply = format!("# {}\n{}", title, description);
                    }
                }

                let _ = command.create_interaction_response(&ctx.http, |r| {
                    r.kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|m| m.content(reply))
                }).await;
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
}