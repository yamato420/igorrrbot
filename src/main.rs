use std::env;
use dotenvy::dotenv;
use poise::serenity_prelude::{self as serenity, *};

mod dbms;
mod ticket;
mod commands;
mod utils;
use dbms::DBMS;

use crate::utils::{get_env_var, Error};


struct Data {
    dbms: DBMS
}



#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok();

    let db_host: String = get_env_var("DB_HOST").await;
    let db_user: String = get_env_var("DB_USER").await;
    let db_password: String = get_env_var("DB_PASSWORD").await;
    let db_name: String = get_env_var("DB_NAME").await;

    let dbms: DBMS = DBMS::new(&format!("host={} user={} password={} dbname={}", db_host, db_user, db_password, db_name)).await?;
    dbms.create_table().await?;

    let token: String = get_env_var("BOT_TOKEN").await;

    let options = poise::FrameworkOptions {
        commands: vec![
            commands::ticket(),
        ],
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("{} is connected! 🫡", _ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    dbms
                })
            })
        })
        .options(options)
        .build();

    let intents: GatewayIntents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILD_MEMBERS;

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.expect("Failed to create client");

    Ok(())
}