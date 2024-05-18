use super::TableEntry;
use banyanfs::api::platform::ApiUserKey;
use cli_table::{Cell, CellStruct};

impl TableEntry for ApiUserKey {
    fn row(&self) -> Vec<CellStruct> {
        vec![
            self.name().cell(),
            self.user_id().cell(),
            self.fingerprint().cell(),
            self.api_access().cell(),
            self.public_key().cell(),
            true.cell(),
        ]
    }

    fn title() -> Vec<CellStruct> {
        vec![
            "Name".cell(),
            "User ID".cell(),
            "Fingerprint".cell(),
            "API".cell(),
            "Public Key".cell(),
            "Persisted Remotely".cell(),
        ]
    }
}
