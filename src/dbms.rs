use tokio_postgres::{Client, NoTls, Error};

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

    pub async fn insert_ticket(&self, author: &str, title: &str, description: &str, is_open: bool) -> Result<u32, Error> {
        let row = self.client.query_one(
            "INSERT INTO tickets
            (author, title, description, is_open)
            VALUES ($1, $2, $3, $4)
            RETURNING id",
            &[&author, &title, &description, &is_open],
        ).await?;

        let id: i32 = row.get(0);
        let id: u32 = id as u32;
        Ok(id)
    }

    pub async fn close_ticket(&self, id: u32) -> Result<(), Error> {
        self.client.execute(
            "
            UPDATE tickets
            SET is_open = false
            WHERE id = ($1)
            ", &[&id]).await?;
        Ok(())
    }

    pub async fn query_tickets(&self) -> Result<Vec<(u32, String)>, Error> {
        let rows = self.client.query("SELECT id, title FROM tickets", &[]).await?;
        let mut tickets = Vec::new();

        for row in rows {
            let id: u32 = row.get(0);
            let title: String = row.get(1);
            tickets.push((id, title));
        }

        Ok(tickets)
    }
}