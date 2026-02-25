use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct Database(pub HashMap<u32, Year>);

impl Database {
    pub fn load() -> Result<Self, String> {
        const DATA: &[u8] = include_bytes!("../data/tables.db");
        postcard::from_bytes(DATA).map_err(|e| format!("Failed to parse table: {e:?}"))
    }
}

#[derive(Deserialize)]
pub struct Year(pub HashMap<String, CantonalBase>);

#[derive(Deserialize)]
pub struct CantonalBase {
    pub rate: f64,
    pub scale: CantonalScale,
}

#[derive(Deserialize)]
pub struct CantonalScale {
    pub splitting: f64,
    pub single: Table,
    pub married: Table,
}

#[derive(Deserialize)]
pub struct Table {
    table: RawTable,
    policy: EvalPolicy,
}

impl Table {
    pub fn eval(&self, x: f64) -> f64 {
        match self.policy {
            EvalPolicy::Raw | EvalPolicy::NoSplitRaw => self.table.eval_raw(x),
            EvalPolicy::Round100 | EvalPolicy::DoubleRound100 | EvalPolicy::NoSplitRound100 => {
                self.table.eval_round100(x)
            }
            EvalPolicy::Valais => self.table.eval_raw(x),
        }
    }

    pub fn eval_split(&self, x: f64, split: f64) -> f64 {
        match self.policy {
            EvalPolicy::Raw => self.table.eval_split_raw(x, split),
            EvalPolicy::Round100 => self.table.eval_split_round100(x, split),
            EvalPolicy::DoubleRound100 => self.table.eval_split_double_round100(x, split),
            EvalPolicy::NoSplitRaw => {
                assert_eq!(split, 0.0);
                self.table.eval_split_raw(x, split)
            }
            EvalPolicy::NoSplitRound100 => {
                assert_eq!(split, 0.0);
                self.table.eval_split_round100(x, split)
            }
            EvalPolicy::Valais => self.table.eval_split_raw(x, split),
        }
    }
}

#[derive(Deserialize)]
pub enum EvalPolicy {
    Raw,
    Round100,
    DoubleRound100,
    NoSplitRaw,
    NoSplitRound100,
    Valais,
}

#[derive(Deserialize)]
enum RawTable {
    Bund(TableBund),
    Flattax(TableFlattax),
    Formel(TableFormel),
    Freiburg(TableFreiburg),
    Zuerich(TableZuerich),
}

impl RawTable {
    fn eval_raw(&self, x: f64) -> f64 {
        match self {
            RawTable::Bund(table) => table.eval(x),
            RawTable::Flattax(table) => table.eval(x),
            RawTable::Formel(table) => table.eval(x),
            RawTable::Freiburg(table) => table.eval(x),
            RawTable::Zuerich(table) => table.eval(x),
        }
    }

    fn eval_round100(&self, x: f64) -> f64 {
        // Round down to multiple of 100 CHF.
        self.eval_raw(Self::floor_100(x))
    }

    fn eval_split_raw(&self, x: f64, split: f64) -> f64 {
        if split == 0.0 {
            self.eval_raw(x)
        } else {
            self.eval_raw(x / split) * split
        }
    }

    fn eval_split_round100(&self, x: f64, split: f64) -> f64 {
        if split == 0.0 {
            self.eval_round100(x)
        } else {
            // Round down to multiple of 100 CHF.
            let xx = Self::floor_100(x);
            let yy = xx / split;
            let rate = if yy == 0.0 {
                0.0
            } else {
                self.eval_raw(yy) / yy
            };
            rate * xx
        }
    }

    fn eval_split_double_round100(&self, x: f64, split: f64) -> f64 {
        if split == 0.0 {
            self.eval_round100(x)
        } else {
            // Round down to multiple of 100 CHF.
            let xx = Self::floor_100(x);
            let yy = Self::floor_100(xx / split);
            let rate = if yy == 0.0 {
                0.0
            } else {
                self.eval_raw(yy) / yy
            };
            rate * xx
        }
    }

    fn floor_100(x: f64) -> f64 {
        (x / 100.0).floor() * 100.0
    }
}

#[derive(Deserialize)]
struct TableBund(Vec<TableBundEntry>);

impl TableBund {
    fn eval(&self, x: f64) -> f64 {
        for entry in self.0.iter().rev() {
            if x >= entry.bracket_start {
                return entry.base_tax + (x - entry.bracket_start) * entry.marginal_rate / 100.0;
            }
        }
        0.0
    }
}

#[derive(Deserialize)]
struct TableBundEntry {
    bracket_start: f64,
    base_tax: f64,
    marginal_rate: f64,
}

#[derive(Deserialize)]
struct TableFlattax(f64);

impl TableFlattax {
    fn eval(&self, x: f64) -> f64 {
        x * self.0 / 100.0
    }
}

#[derive(Deserialize)]
struct TableFormel(Vec<TableFormelEntry>);

impl TableFormel {
    fn eval(&self, x: f64) -> f64 {
        for entry in self.0.iter().rev() {
            if x >= entry.bracket_start {
                return entry.formula.eval(x);
            }
        }
        0.0
    }
}

#[derive(Deserialize)]
struct TableFormelEntry {
    bracket_start: f64,
    formula: Formula,
}

#[derive(Deserialize)]
pub enum Formula {
    Input,
    Const(f64),
    Log(Box<Formula>),
    Add(Box<Formula>, Box<Formula>),
    Sub(Box<Formula>, Box<Formula>),
    Mul(Box<Formula>, Box<Formula>),
    Div(Box<Formula>, Box<Formula>),
}

impl Formula {
    pub fn eval(&self, x: f64) -> f64 {
        match self {
            Formula::Input => x,
            Formula::Const(c) => *c,
            Formula::Log(f) => f.eval(x).ln(),
            Formula::Add(f, g) => f.eval(x) + g.eval(x),
            Formula::Sub(f, g) => f.eval(x) - g.eval(x),
            Formula::Mul(f, g) => f.eval(x) * g.eval(x),
            Formula::Div(f, g) => f.eval(x) / g.eval(x),
        }
    }
}
#[derive(Deserialize)]
struct TableFreiburg(Vec<TableFreiburgEntry>);

impl TableFreiburg {
    fn eval(&self, x: f64) -> f64 {
        for (i, entry) in self.0.iter().enumerate().rev() {
            if x >= entry.bracket_start {
                let tax_rate = if i + 1 == self.0.len() {
                    entry.tax_rate
                } else {
                    let weight = (x - entry.bracket_start)
                        / (self.0[i + 1].bracket_start - entry.bracket_start);
                    entry.tax_rate + weight * (self.0[i + 1].tax_rate - entry.tax_rate)
                };
                return x * tax_rate / 100.0;
            }
        }
        0.0
    }
}

#[derive(Deserialize)]
struct TableFreiburgEntry {
    bracket_start: f64,
    tax_rate: f64,
}

#[derive(Deserialize)]
struct TableZuerich(Vec<TableZuerichEntry>);

impl TableZuerich {
    fn eval(&self, mut x: f64) -> f64 {
        let mut tax = 0.0;
        for entry in &self.0 {
            if x <= entry.bracket_len {
                tax += x * entry.marginal_rate / 100.0;
                break;
            } else {
                tax += entry.bracket_len * entry.marginal_rate / 100.0;
                x -= entry.bracket_len;
            }
        }
        tax
    }
}

#[derive(Deserialize)]
struct TableZuerichEntry {
    bracket_len: f64,
    marginal_rate: f64,
}
