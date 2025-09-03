use tokio_postgres::{Client, NoTls, Error};

use crate::ticket::Ticket;

pub struct DBMS {
    client: Client,
}

impl DBMS {
    pub async fn new(connection_string: &str) -> Result<Self, Error> {
        let (client, connection) = tokio_postgres::connect(connection_string, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        Ok(DBMS { client })
    }

    pub async fn create_table(&self) -> Result<(), Error> {
        self.client.execute(
            "CREATE TABLE IF NOT EXISTS tickets (
                id SERIAL PRIMARY KEY,
                author VARCHAR NOT NULL,
                title VARCHAR NOT NULL,
                description VARCHAR NOT NULL,
                is_open BOOLEAN NOT NULL
                )",
            &[],
        ).await?;
        Ok(())
    }

    pub async fn insert_ticket(&self, author: &str, title: &str, description: &str) -> Result<u32, Error> {
        let row = self.client.query_one(
            "INSERT INTO tickets
            (author, title, description, is_open)
            VALUES ($1, $2, $3, true)
            RETURNING id
            ", &[&author, &title, &description],
        ).await?;

        let id: i32 = row.get(0);
        let id: u32 = id as u32;
        Ok(id)
    }

    pub async fn close_ticket(&self, id: u32) -> Result<(), Error> {
        let a: i32 = id as i32;
        self.client.execute(
            "
            UPDATE tickets
            SET is_open = false
            WHERE id = ($1)
            ", &[&a]).await?;
        Ok(())
    }

    pub async fn get_open_tickets(&self) -> Result<Vec<Ticket>, Error> {
        let rows = self.client.query("
            SELECT * FROM tickets
            WHERE is_open = true
            ", &[]).await?;

        let mut tickets: Vec<Ticket> = Vec::new();

        for row in rows {
            let id: i32 = row.get(0);
            let id: u32 = id as u32;

            tickets.push(Ticket {
                id: id,
                author: row.get(1),
                title: row.get(2),
                description: row.get(3),
                is_open: row.get(4)
            });
        }

        Ok(tickets)
    }
}