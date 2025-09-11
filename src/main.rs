use std::env;
use dotenvy::dotenv;
use poise::serenity_prelude::{self as serenity, *};

mod dbms;
mod ticket;
mod commands;
mod utils;
use dbms::DBMS;


type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

struct Data {
    dbms: DBMS
}



#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok();

    let db_host: String = env::var("DB_HOST").expect("Couldn't find DB_HOST environment variable");
    let db_user: String = env::var("DB_USER").expect("Couldn't find DB_USER environment variable");
    let db_password: String = env::var("DB_PASSWORD").expect("Couldn't find DB_PASSWORD environment variable");
    let db_name: String = env::var("DB_NAME").expect("Couldn't find DB_NAME environment variable");

    let dbms: DBMS = DBMS::new(&format!("host={} user={} password={} dbname={}", db_host, db_user, db_password, db_name)).await?;
    dbms.create_table().await?;

    let token: String = env::var("BOT_TOKEN").expect("Couldn't find BOT_TOKEN environment variable");

    let options = poise::FrameworkOptions {
        commands: vec![
            commands::ticket(),
            commands::open(),
            commands::close(),
            commands::show(),
            commands::list(),
            commands::listall(),
        ],
        command_check: Some(|ctx: Context| {
            Box::pin(async move {
                println!("{:?}", ctx.command().checks);
                // if ctx.command().checks {
                //     return Ok(false);
                // }
                Ok(true)
            })
        }),
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