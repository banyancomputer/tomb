use banyanfs::api::platform::ApiDrive;
use cli_table::{Cell, CellStruct};

use crate::cli::display::Persistence;

use super::TableEntry;

impl TableEntry for ApiDrive {
    fn row(&self) -> Vec<CellStruct> {
        vec![
            self.name.clone().cell(),
            self.id.clone().cell(),
            Persistence::RemoteOnly.cell(),
        ]
    }

    fn title() -> Vec<CellStruct> {
        vec!["Name".cell(), "ID".cell(), "Persistence".cell()]
    }
}
