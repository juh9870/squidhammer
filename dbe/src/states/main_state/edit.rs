use camino::Utf8PathBuf;
use undo::{Edit, Merged};

use crate::states::main_state::state::EditorData;
use crate::value::EValue;

/// DBE edit history
///
/// Each edit is supposed to clean up after itself in case of a failure, to
/// avoid state corruption
#[derive(Debug)]
pub(super) enum MainStateEdit {
    DeleteLastEdit,
    CreateFile(EValue, Utf8PathBuf),
}

impl Edit for MainStateEdit {
    type Target = EditorData;
    type Output = anyhow::Result<()>;

    fn edit(&mut self, target: &mut Self::Target) -> Self::Output {
        match self {
            MainStateEdit::CreateFile(value, path) => {
                target.fs.new_item(value.clone().into(), path)
            }
            MainStateEdit::DeleteLastEdit => Ok(()),
        }
    }

    fn undo(&mut self, target: &mut Self::Target) -> Self::Output {
        match self {
            MainStateEdit::DeleteLastEdit => Ok(()),
            MainStateEdit::CreateFile(_, path) => {
                target.fs.delete_entry(path);
                Ok(())
            }
        }
    }

    fn merge(&mut self, other: Self) -> Merged<Self>
    where
        Self: Sized,
    {
        if matches!(self, MainStateEdit::DeleteLastEdit)
            || matches!(other, MainStateEdit::DeleteLastEdit)
        {
            Merged::Annul
        } else {
            Merged::No(other)
        }
    }
}
