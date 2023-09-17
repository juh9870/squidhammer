use indent::indent_by;
use itertools::Itertools;

pub fn display_error(error: impl Into<anyhow::Error>) -> String {
    let error = error.into();
    let mut chain = error
        .chain()
        .rev()
        .map(|e| format!("```\n{}\n```", e.to_string().trim()));
    let Some(mut main_error) = chain.next() else {
        return "Empty error message".to_string();
    };

    main_error += "\n## Stacktrace:\n\n";
    main_error += &chain
        .enumerate()
        .map(|(i, e)| format!("{}. {}", i + 1, indent_by(3, e)))
        .join("\n");
    #[cfg(target_os="windows")] {
        main_error = main_error.replace(r"\\?\", "");
    }
    main_error
}

#[macro_export]
macro_rules! somehow {
    ($body:block) => {
        (|| Result::<_, anyhow::Error>::Ok($body))()
    };
}

pub use somehow;
