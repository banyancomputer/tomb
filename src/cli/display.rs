use banyanfs::api::platform::{ApiUserKey, ApiUserKeyAccess};
use cli_table::{format::Justify, Cell, CellStruct, Style, Table, TableDisplay};

pub trait TableEntry: Sized {
    fn row(&self) -> Vec<CellStruct>;
    fn title() -> Vec<CellStruct>;
}

pub trait TableAble {
    fn entries(&self) -> Vec<Vec<CellStruct>>;
    fn display_table(&self) -> Result<TableDisplay, std::io::Error>;
}

impl<T> TableAble for Vec<T>
where
    T: TableEntry,
{
    fn display_table(&self) -> Result<TableDisplay, std::io::Error> {
        display_table(self.entries(), T::title())
    }

    fn entries(&self) -> Vec<Vec<CellStruct>> {
        self.iter()
            .map(|item| item.row())
            .collect::<Vec<Vec<CellStruct>>>()
    }
}

pub fn display_table(
    entries: Vec<Vec<CellStruct>>,
    title: Vec<CellStruct>,
) -> Result<TableDisplay, std::io::Error> {
    Vec::<Vec<CellStruct>>::table(entries)
        .title(title)
        .bold(true)
        .display()
}

impl TableEntry for ApiUserKey {
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

/*
impl Display for ApiUserKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
*/
