use serenity::{
    client::Client,
    framework::standard::StandardFramework, prelude::GatewayIntents,
};
use std::env;
use dotenvy::dotenv;
use tokio_postgres::{Error};

mod dbms;
mod commands;
mod ticket;

use commands::GENERAL_GROUP;

use crate::{commands::*, dbms::DBMS};


#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok();

    let db_host: String = env::var("DB_HOST").expect("Couldn't find DB_HOST environment variable.");
    let db_user: String = env::var("DB_USER").expect("Couldn't find DB_USER environment variable.");
    let db_password: String = env::var("DB_PASSWORD").expect("Couldn't find DB_PASSWORD environment variable.");
    let db_name: String = env::var("DB_NAME").expect("Couldn't find DB_NAME environment variable.");

    let dbms: DBMS = DBMS::new(&format!("host={} user={} password={} dbname={}", db_host, db_user, db_password, db_name)).await?;
    dbms.create_table().await?;

    // Required env vars: BOT_TOKEN, TEST_GUILD_ID, TEST_MOD_ROLE_ID
    let token: String = env::var("BOT_TOKEN").expect("Couldn't find BOT_TOKEN environment variable.");
    let handler: Handler = Handler { dbms };
    let framework: StandardFramework = StandardFramework::new().configure(|c| c.prefix("!")).group(&GENERAL_GROUP);

    let intents: GatewayIntents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILDS | GatewayIntents::DIRECT_MESSAGES;
    let mut client: Client = Client::builder(&token, intents)
        .framework(framework)
        .event_handler(handler)
        .await
        .expect("Couldn't create client.");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }

    Ok(())
}
