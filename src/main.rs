use serenity::{
    client::Client,
    framework::standard::StandardFramework, prelude::GatewayIntents,
};
use std::env;
use dotenvy::dotenv;
use tokio_postgres::{Error};

mod dbms;
mod commands;

use commands::GENERAL_GROUP;

use crate::{commands::*, dbms::DBMS};


#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok();

    let dbms = DBMS::new("host=localhost user=postgres password=monkers dbname=igorrrbot").await?;
    dbms.create_table().await?;

    // Required env vars: BOT_TOKEN, TEST_GUILD_ID
    let token = env::var("BOT_TOKEN").expect("Couldn't find BOT_TOKEN environment variable.");
    let handler = Handler { dbms };
    let framework = StandardFramework::new().configure(|c| c.prefix("!")).group(&GENERAL_GROUP);

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILDS | GatewayIntents::DIRECT_MESSAGES;
    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .event_handler(handler)
        .await
        .expect("Couldn't create client.");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }

    Ok(())
}
