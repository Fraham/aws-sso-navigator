use skim::prelude::*;
use std::io::Cursor;

pub fn skim_pick(prompt: &str, options: Vec<String>) -> Option<String> {
    let input = options.join("\n");
    let prompt_str = format!("{}> ", prompt);
    let options = SkimOptionsBuilder::default()
        .height(String::from("30%"))
        .prompt(prompt_str)
        .multi(false)
        .bind(vec!["esc:abort".to_string()])
        .no_mouse(true)
        .build()
        .unwrap();

    let item_reader = SkimItemReader::default();
    let items = item_reader.of_bufread(Cursor::new(input));
    let output = Skim::run_with(&options, Some(items))?;

    (!output.is_abort && !output.selected_items.is_empty())
        .then(|| output.selected_items[0].output().to_string())
}
