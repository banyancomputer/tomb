use banyanfs::api::platform::ApiDrive;
use cli_table::{Cell, CellStruct};

use super::TableEntry;

impl TableEntry for ApiDrive {
    fn row(&self) -> Vec<CellStruct> {
        vec![self.id.clone().cell(), self.name.clone().cell()]
    }

    fn title() -> Vec<CellStruct> {
        vec!["ID".cell(), "Name".cell()]
    }
}
