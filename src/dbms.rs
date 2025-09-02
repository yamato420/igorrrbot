use tokio_postgres::{Client, NoTls, Error};

pub struct DBMS {
    client: Client,
}

impl DBMS {
    pub async fn new(connection_string: &str) -> Result<Self, Error> {
        let (client, connection) = tokio_postgres::connect(connection_string, NoTls).await?;

        // Spawn a new task to handle the connection
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

    pub async fn insert_ticket(&self, author: &str, title: &str, description: &str, is_open: bool) -> Result<(), Error> {
        self.client.execute(
            "INSERT INTO tickets (author, title, description, is_open) VALUES ($1, $2, $3, $4)",
            &[&author, &title, &description, &is_open],
        ).await?;
        Ok(())
    }

    pub async fn query_tickets(&self) -> Result<Vec<(i32, String)>, Error> {
        let rows = self.client.query("SELECT id, title FROM tickets", &[]).await?;
        let mut tickets = Vec::new();

        for row in rows {
            let id: i32 = row.get(0);
            let title: String = row.get(1);
            tickets.push((id, title));
        }

        Ok(tickets)
    }
}