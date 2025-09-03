use serenity::{
    async_trait,
    framework::standard::macros::group,
    model::{
        application::{command::CommandOptionType,
        interaction::{application_command::ApplicationCommandInteraction, Interaction, InteractionResponseType}}, gateway::Ready, guild::Member, id::{GuildId, RoleId}
    },
    prelude::*,
};
use dotenvy::dotenv;
use std::env;

use crate::dbms::DBMS;

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
                let mod_role_id: String = env::var("TEST_MOD_ROLE_ID").expect("Couldn't find TEST_MOD_ROLE_ID environment variable.");

                let guild_id: GuildId = command.guild_id.unwrap();
                let user_id: serenity::model::prelude::UserId = command.user.id;
                let member: serenity::model::prelude::Member = guild_id.member(&ctx.http, user_id).await.unwrap();

                if Handler::has_role(&member, RoleId(mod_role_id.parse::<u64>().unwrap())).await {
                    let id: u32 = self.get_option(&command, "id").await.parse::<u32>().unwrap_or(u32::max_value());

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
}