use camino::Utf8PathBuf;
use undo::{Edit, Merged};

use crate::states::main_state::EditorState;
use crate::value::etype::registry::ETypetId;

/// DBE edit history
///
/// Each edit is supposed to clean up after itself in case of a failure, to
/// avoid state corruption
#[derive(Debug)]
pub(super) enum MainStateEdit {
    DeleteLastEdit,
    CreateFile(ETypetId, Utf8PathBuf),
    CreateFolder(Utf8PathBuf),
}

impl Edit for MainStateEdit {
    type Target = EditorState;
    type Output = anyhow::Result<()>;

    fn edit(&mut self, target: &mut Self::Target) -> Self::Output {
        match self {
            MainStateEdit::CreateFile(type_id, path) => target.new_item(type_id, path),
            MainStateEdit::CreateFolder(path) => target.new_folder(path),
            MainStateEdit::DeleteLastEdit => Ok(()),
        }
    }

    fn undo(&mut self, target: &mut Self::Target) -> Self::Output {
        match self {
            MainStateEdit::DeleteLastEdit => Ok(()),
            MainStateEdit::CreateFile(_, path) | MainStateEdit::CreateFolder(path) => {
                target.delete_file(path)?;
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
