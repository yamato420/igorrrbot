pub struct Ticket {
    pub id: u64,
    pub author: u64,
    pub title: String,
    pub description: String,
    pub is_open: bool,
    pub channel_id: u64
}