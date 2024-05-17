use std::fmt::Display;

pub trait TableAble: Sized {
    fn row(&self) -> Vec<CellStruct>;
    fn title() -> Vec<CellStruct>;
    fn table(items: Vec<Self>) -> Result<TableDisplay, std::io::Error> {
        let table = items
            .into_iter()
            .map(|item| item.row())
            .collect::<Vec<Vec<CellStruct>>>()
            .table()
            .title(Self::title())
            .bold(true);
        table.display()
    }
}

impl TableAble for ApiUserKey {
    fn row(&self) -> Vec<CellStruct> {
        vec![
            self.name().cell(),
            self.id().cell(),
            self.fingerprint().cell(),
            self.api_access().cell(),
            self.public_key().cell(),
        ]
    }

    fn title() -> Vec<CellStruct> {
        vec![
            "Name".cell(),
            "User ID".cell(),
            "Fingerprint".cell(),
            "API".cell(),
            "Public Key".cell(),
        ]
    }
}

fn printmeee() {
    let table = vec![
        vec!["Tom".cell(), 10.cell().justify(Justify::Right)],
        vec!["Jerry".cell(), 15.cell().justify(Justify::Right)],
        vec!["Scooby Doo".cell(), 20.cell().justify(Justify::Right)],
    ]
    .table()
    .title(vec![
        "Name".cell().bold(true),
        "Age (in years)".cell().bold(true),
    ])
    .bold(true);
}

use banyanfs::api::platform::{ApiUserKey, ApiUserKeyAccess};
use cli_table::{format::Justify, Cell, CellStruct, Style, Table, TableDisplay};

/*
impl Display for ApiUserKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
*/
