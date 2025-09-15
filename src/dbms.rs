use tokio_postgres::{Client, Error, NoTls, Row};

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
            "
            CREATE TABLE IF NOT EXISTS tickets (
            id SERIAL PRIMARY KEY,
            author VARCHAR NOT NULL,
            title VARCHAR NOT NULL,
            description VARCHAR NOT NULL,
            is_open BOOLEAN NOT NULL,
            channel_id VARCHAR NOT NULL
            )",
            &[],
        ).await?;
        Ok(())
    }

    pub async fn insert_ticket(&self, author: u64, title: &str, description: &str) -> Result<u64, Error> {
        let row: Row = self.client.query_one(
            "
            INSERT INTO tickets
            (author, title, description, is_open, channel_id)
            VALUES ($1, $2, $3, true, 0)
            RETURNING id
            ", 
            &[&author.to_string(), &title, &description],
        ).await?;

        let id: i32 = row.get(0);
        let id: u64 = id as u64;
        Ok(id)
    }

    pub async fn set_channel_id(&self, id: i32, channel_id: String) -> Result<(), Error> {
        self.client.execute(
            "
            UPDATE tickets
            SET channel_id = $1
            WHERE id = $2
            ",
            &[&channel_id, &id]
        ).await?;

        Ok(())
    }

    pub async fn close_ticket(&self, id: u64) -> Result<bool, Error> {
        let id: i32 = id as i32;
        let rows: u64 = self.client.execute(
            "
            UPDATE tickets
            SET is_open = false
            WHERE id = $1 AND is_open = true
            ",
            &[&id]
        ).await?;

        Ok(rows > 0)
    }

    pub async fn get_tickets(&self, show_only_open_tickets: bool) -> Result<Vec<Ticket>, Error> {
        let statement: &str = if show_only_open_tickets {
            "
            SELECT * FROM tickets
            WHERE is_open = true
            "
        } else {
            "
            SELECT * FROM tickets
            "
        };

        let rows: Vec<Row> = self.client.query(statement, &[]).await?;
        let mut tickets: Vec<Ticket> = Vec::new();

        for row in rows {
            let id: u64 = row.get::<_, i32>(0) as u64;
            let author: u64 = row.get::<_, String>(1).parse::<u64>().expect("author must be u64");
            let channel_id: u64 = row.get::<_, String>(5).parse::<u64>().expect("channel_id must be u64");

            tickets.push(Ticket {
                id,
                author,
                title: row.get(2),
                description: row.get(3),
                is_open: row.get(4),
                channel_id
            });
        }

        Ok(tickets)
    }

    pub async fn get_channel_id(&self, id: i32) -> Result<u64, Error> {
        let row: Row = self.client.query_one(
            "
            SELECT channel_id FROM tickets
            WHERE id = $1
            ",
            &[&id]
        ).await?;

        let channel_id: String = row.get(0);
        let channel_id: u64 = channel_id.parse::<u64>().expect("channel_id must be u64");

        Ok(channel_id)
    }
}