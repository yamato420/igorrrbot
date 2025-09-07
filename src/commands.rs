use regex::Regex;
use serenity::{
    async_trait, builder::CreateApplicationCommandPermissionData, framework::standard::macros::group, model::{
        application::{command::CommandOptionType,
        interaction::{application_command::ApplicationCommandInteraction, Interaction, InteractionResponseType}}, gateway::Ready, guild::{Guild, Member}, id::{ChannelId, CommandId, GuildId, RoleId, UserId}, prelude::{command::CommandPermissionType, ChannelType, GuildChannel, PermissionOverwrite, PermissionOverwriteType}, Permissions
    }, prelude::*
};
use dotenvy::dotenv;
use std::{env, str::FromStr};

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
        dotenv().ok();

        println!("{} is connected!", ready.user.name);

        let guild_id: GuildId = GuildId(
            Self::get_env_var("TEST_GUILD_ID")
                .await
                .parse::<u64>()
                .expect("TEST_GUILD_ID must be a u64"),
        );
        let mod_role_id: RoleId = RoleId(
            Self::get_env_var("TEST_MOD_ROLE_ID")
                .await
                .parse::<u64>()
                .expect("TEST_MOD_ROLE_ID must be a u64")
        );
        let mut mod_commands: Vec<CommandId> = Vec::new();

        let _ = guild_id.create_application_command(&ctx.http, |cmd| {
            cmd.name("help")
                .description("Show useful commands")
        });

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
                .create_option(|o| {
                    o.name("related_users")
                    .description("Related users (@ping them)")
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

        mod_commands.push(guild_id.create_application_command(&ctx.http, |cmd| {
            cmd.name("list")
                .description("[MODS ONLY] List open tickets")
        }).await.unwrap().id);

        mod_commands.push(guild_id.create_application_command(&ctx.http, |cmd| {
            cmd.name("listall")
                .description("[MODS ONLY] List all tickets")
        }).await.unwrap().id);

        for cmd in mod_commands {
            let _ = guild_id.create_application_command_permission(&ctx.http, cmd, |builder| {
                let mut mod_perm: CreateApplicationCommandPermissionData = CreateApplicationCommandPermissionData::default();
                let mut everyone_perm: CreateApplicationCommandPermissionData = CreateApplicationCommandPermissionData::default();

                mod_perm
                    .id(mod_role_id.0)
                    .kind(CommandPermissionType::Role)
                    .permission(true);

                builder.add_permission(mod_perm);

                everyone_perm
                    .id(guild_id.into())
                    .kind(CommandPermissionType::Role)
                    .permission(false);

                builder.add_permission(everyone_perm)
            }).await;
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            self.handle_application_command(&ctx, &command).await;
        }
    }
}

impl Handler {
    pub async fn handle_application_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) {
        dotenv().ok();
        let mut reply: String = String::new();

        match command.data.name.as_str() {
            "open" => {
                let author: String = command.user.id.to_string();
                let title: String = String::from(self.get_option(&command, "title").await.trim_matches('"'));
                let description: String = String::from(self.get_option(&command, "description").await.trim_matches('"'));
                let related_users_option: String = String::from(self.get_option(&command, "related_users").await.trim_matches('"'));
                let related_users: Vec<&str> = related_users_option.split(" ").collect();

                match self.dbms.insert_ticket(&author, &title, &description).await {
                    Ok(id) => {
                        let ticket: Ticket = Ticket {
                            id: id,
                            author: author,
                            title: title.clone(),
                            description: description,
                            is_open: true
                        };

                        let guild_id: GuildId = command.guild_id.unwrap();
                        let category_id: ChannelId = ChannelId::from_str(
                            &Self::get_env_var("TEST_CATEGORY_ID")
                            .await)
                            .unwrap();
                        let channel_name: String = format!("(#{}): {}", &ticket.id, &ticket.title);
                        let mut allowed_users: Vec<UserId> = Vec::new();
                        allowed_users.push(command.user.id);
                        
                        let user_id_regex: String = String::from(r"<@!?(\d{17,19})>");
                        let regex: Regex = Regex::new(&user_id_regex).unwrap();

                        for user in related_users {
                            if regex.is_match(user) {
                                let user_id: UserId = match UserId::from_str(user) {
                                    Ok(id) => id,
                                    Err(e) => {
                                        eprintln!("{}", e);
                                        return
                                    }
                                };
                                allowed_users.push(user_id);
                            }
                        }

                        let channel: ChannelId = Self::create_ticket_channel(guild_id, category_id, &channel_name, allowed_users, &ctx).await.expect("Failed to create channel");
                        let _ = channel.say(&ctx.http, Self::display_ticket(&ticket).await).await;

                        reply = format!("Opened ticket (#{}): {}.", id, title);
                        println!("{}", reply);
                    },
                    Err(e) => {
                        reply = format!("Failed to open ticket {}.", title);
                        eprintln!("{}\n{}", reply, e);
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
                            println!("{}", reply);
                        },
                        Err(e) => {
                            reply = format!("Failed to close ticket #{}.", id);
                            eprintln!("{}\n{}", reply, e);
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

                let tickets: Vec<Ticket> = self.dbms.get_tickets(false).await.expect("Failed to get tickets");

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
                    let tickets: Vec<Ticket> = self.dbms.get_tickets(true).await.expect("Failed to get tickets");

                    for ticket in tickets {
                        reply = format!("{}\n(#{}): {}", reply, ticket.id, ticket.title);
                    }
                } else {
                    reply = String::from("Only mods can list open tickets.\nThis incident will be reported.");
                }
            }

            "listall" => {
                if Self::is_mod(&ctx, &command).await {
                    let tickets: Vec<Ticket> = self.dbms.get_tickets(false).await.expect("Failed to get tickets");

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

    async fn create_ticket_channel(guild_id: GuildId, category_id: ChannelId, channel_name: &str, allowed_users: Vec<UserId>, ctx: &Context) -> Result<ChannelId, Box<dyn std::error::Error>> {
        dotenv().ok();

        let mod_role_id: RoleId = RoleId(
            Self::get_env_var("TEST_MOD_ROLE_ID")
                .await
                .parse::<u64>()
                .expect("TEST_MOD_ROLE_ID must be a u64")
        );

        let guild: Guild = guild_id.to_guild_cached(&ctx.cache).unwrap();
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
            kind: PermissionOverwriteType::Role(mod_role_id)
        });

        overwrites.push(PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
            kind: PermissionOverwriteType::Role(RoleId(*guild_id.as_u64()))
        });     

        let channel: GuildChannel = guild.create_channel(&ctx.http, |c| {
            c.name(channel_name)
            .kind(ChannelType::Text)
            .category(category_id)
            .permissions(overwrites)
        }).await?;

        Ok(channel.id)
    }

    async fn get_env_var(var: &str) -> String {
        env::var(&var).expect(
            &format!("Couldn't find {} environment variable", var)
        )
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
        let mod_role_id: RoleId = RoleId(
            Self::get_env_var("TEST_MOD_ROLE_ID")
                .await
                .parse::<u64>()
                .expect("TEST_MOD_ROLE_ID must be a u64")
        );

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