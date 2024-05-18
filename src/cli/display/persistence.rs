use std::fmt::Display;

#[derive(Debug)]
pub enum Persistence {
    LocalOnly,
    RemoteOnly,
    Sync,
}

impl Display for Persistence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Persistence::LocalOnly => f.write_str("Local Only"),
            Persistence::RemoteOnly => f.write_str("Remote Only"),
            Persistence::Sync => f.write_str("Sync"),
        }
    }
}
