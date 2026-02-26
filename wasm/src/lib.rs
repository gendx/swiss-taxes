#![forbid(unsafe_code)]

mod plot;
mod table;

use plot::plot_income_tax_diff;
use table::Database;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::{HtmlCanvasElement, console};

#[wasm_bindgen]
pub struct State {
    db: Option<Database>,
}

#[wasm_bindgen]
impl State {
    #[expect(clippy::new_without_default)]
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let db = match Database::load() {
            Ok(db) => Some(db),
            Err(e) => {
                console::error_1(&JsValue::from_str(&format!("Failed to load data: {e:?}")));
                None
            }
        };
        Self { db }
    }

    pub fn plot(
        &self,
        canvas: HtmlCanvasElement,
        year: u32,
        canton: &str,
        max_salary: i32,
    ) -> Result<(), JsValue> {
        match &self.db {
            None => Err("Failed to load data".into()),
            Some(db) => {
                let entry = db
                    .db
                    .get(&year)
                    .ok_or_else(|| format!("Didn't find year: {year}"))?
                    .0
                    .get(canton)
                    .ok_or_else(|| format!("Didn't find canton: {canton}"))?;
                let scale = &db.arena[entry.scale_index as usize];
                plot_income_tax_diff(
                    canvas,
                    max_salary,
                    entry.rate,
                    scale.splitting,
                    &scale.single,
                    &scale.married,
                )?;
                Ok(())
            }
        }
    }
}
