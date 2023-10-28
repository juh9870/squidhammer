use crate::dbe_files::EditorItem;
use crate::editable::EditableFile;
use anyhow::{bail, Context};
use camino::Utf8PathBuf;
use undo::{Edit, Merged};

use crate::states::main_state::state::EditorData;

/// DBE edit history
///
/// Each edit is supposed to clean up after itself in case of a failure, to
/// avoid state corruption
#[derive(Debug)]
pub(super) enum MainStateEdit {
    DeleteLastEdit,
    CreateFile(EditableFile, Utf8PathBuf),
    EditFile {
        path: Utf8PathBuf,
        old: Option<EditorItem>,
        new: EditableFile,
    },
}

impl Edit for MainStateEdit {
    type Target = EditorData;
    type Output = anyhow::Result<()>;

    fn edit(&mut self, target: &mut Self::Target) -> Self::Output {
        match self {
            MainStateEdit::CreateFile(value, path) => target
                .fs
                .new_item(value.clone().into(), path)
                .with_context(|| format!("While creating file at {path}")),
            MainStateEdit::DeleteLastEdit => Ok(()),
            MainStateEdit::EditFile { new, old, path } => {
                *old = Some(
                    target
                        .fs
                        .replace_entry(path, EditorItem::Value(new.clone()))
                        .with_context(|| format!("While editing file at {path}"))?,
                );
                Ok(())
            }
        }
    }

    fn undo(&mut self, target: &mut Self::Target) -> Self::Output {
        match self {
            MainStateEdit::DeleteLastEdit => Ok(()),
            MainStateEdit::CreateFile(_, path) => {
                target.fs.delete_entry(path);
                Ok(())
            }
            MainStateEdit::EditFile { path, old, .. } => {
                let Some(old) = old else {
                    bail!("Can't reverse edit because it was not yet applied")
                };
                target
                    .fs
                    .replace_entry(path, old.clone())
                    .with_context(|| format!("While reverting edit of a file at {path}"))?;
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
