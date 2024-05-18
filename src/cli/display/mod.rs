use cli_table::{print_stdout, CellStruct, Style, Table};
mod keys;

pub trait TableEntry: Sized {
    fn row(&self) -> Vec<CellStruct>;
    fn title() -> Vec<CellStruct>;
}

pub trait TableAble {
    fn entries(&self) -> Vec<Vec<CellStruct>>;
    fn display_table(&self) -> Result<(), std::io::Error>;
}

impl<T> TableAble for Vec<T>
where
    T: TableEntry,
{
    fn display_table(&self) -> Result<(), std::io::Error> {
        let table = self.entries().table().title(T::title()).bold(true);
        print_stdout(table)
    }

    fn entries(&self) -> Vec<Vec<CellStruct>> {
        self.iter()
            .map(|item| item.row())
            .collect::<Vec<Vec<CellStruct>>>()
    }
}

/*
impl<A, B> TableAble for (Vec<A>, Vec<B>)
where
    A: TableEntry,
    B: TableEntry,
{
    fn display_table(&self) -> Result<TableDisplay, std::io::Error> {
        Vec::<Vec<CellStruct>>::table(self.entries())
            .title(A::title())
            .bold(true)
            .display()
    }

    fn entries(&self) -> Vec<Vec<CellStruct>> {
        let mut entries = Vec::new();
        for a in self.0.iter() {
            entries.push(a.row());
        }
        for b in self.1.iter() {
            entries.push(b.row());
        }
        entries
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
*/
