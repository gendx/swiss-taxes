use crate::formula::Formula;
use crate::schema::{Scale, ScaleEntry, TableType};
use anyhow::anyhow;
use log::{debug, warn};
use serde::Serialize;
use std::convert::TryFrom;

#[derive(Debug, Clone, Serialize)]
pub struct Table {
    table: RawTable,
    policy: EvalPolicy,
}

impl Table {
    pub fn try_from(scale: &Scale, policy: EvalPolicy) -> anyhow::Result<Self> {
        Ok(Self {
            table: RawTable::try_from(scale)?,
            policy,
        })
    }

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

#[derive(Debug, Clone, Copy, Serialize)]
pub enum EvalPolicy {
    Raw,
    Round100,
    DoubleRound100,
    NoSplitRaw,
    NoSplitRound100,
    Valais,
}

#[derive(Debug, Clone, Serialize)]
enum RawTable {
    Bund(TableBund),
    Flattax(TableFlattax),
    Formel(TableFormel),
    Freiburg(TableFreiburg),
    Zuerich(TableZuerich),
}

impl TryFrom<&Scale> for RawTable {
    type Error = anyhow::Error;

    fn try_from(scale: &Scale) -> Result<Self, Self::Error> {
        match scale.table_type {
            TableType::Bund => {
                let table = TableBund::try_from(scale.table.as_slice())?;
                Ok(RawTable::Bund(table))
            }
            TableType::Flattax => {
                let table = TableFlattax::try_from(scale.table.as_slice())?;
                Ok(RawTable::Flattax(table))
            }
            TableType::Formel => {
                let table = TableFormel::try_from(scale.table.as_slice())?;
                Ok(RawTable::Formel(table))
            }
            TableType::Freiburg => {
                let table = TableFreiburg::try_from(scale.table.as_slice())?;
                Ok(RawTable::Freiburg(table))
            }
            TableType::Zuerich => {
                let table = TableZuerich::try_from(scale.table.as_slice())?;
                Ok(RawTable::Zuerich(table))
            }
            TableType::Unknown => Err(anyhow!("Unsupported table type: {:?}", scale.table_type)),
        }
    }
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

#[derive(Debug, Clone, Serialize)]
struct TableBund(Vec<TableBundEntry>);

impl TryFrom<&[ScaleEntry]> for TableBund {
    type Error = anyhow::Error;

    fn try_from(table: &[ScaleEntry]) -> Result<Self, Self::Error> {
        Ok(TableBund(
            table
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    if !entry.formula.is_empty() {
                        Err(anyhow!(
                            "Non-empty formula in table of type Bund: {:?}",
                            entry.formula
                        ))
                    } else {
                        if i == 0 && entry.amount != 0.0 {
                            warn!("No entry found for 0 in table of type Bund");
                        }
                        Ok(TableBundEntry {
                            bracket_start: entry.amount,
                            base_tax: entry.taxes,
                            marginal_rate: entry.percent,
                        })
                    }
                })
                .try_collect()?,
        ))
    }
}

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

#[derive(Debug, Clone, Copy, Serialize)]
struct TableBundEntry {
    bracket_start: f64,
    base_tax: f64,
    marginal_rate: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct TableFlattax(f64);

impl TryFrom<&[ScaleEntry]> for TableFlattax {
    type Error = anyhow::Error;

    fn try_from(table: &[ScaleEntry]) -> Result<Self, Self::Error> {
        if table.len() != 1 {
            Err(anyhow!(
                "Table of type Flattax doesn't have size 1: {}",
                table.len()
            ))
        } else if !table[0].formula.is_empty() {
            Err(anyhow!(
                "Non-empty formula in table of type Flattax: {:?}",
                table[0].formula
            ))
        } else if table[0].amount != 0.0 {
            Err(anyhow!(
                "Non-empty amount in table of type Flattax: {}",
                table[0].amount
            ))
        } else {
            let rate = table[0].percent;
            Ok(TableFlattax(rate))
        }
    }
}

impl TableFlattax {
    fn eval(&self, x: f64) -> f64 {
        x * self.0 / 100.0
    }
}

#[derive(Debug, Clone, Serialize)]
struct TableFormel(Vec<TableFormelEntry>);

impl TryFrom<&[ScaleEntry]> for TableFormel {
    type Error = anyhow::Error;

    fn try_from(table: &[ScaleEntry]) -> Result<Self, Self::Error> {
        Ok(TableFormel(
            table
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    debug!("Table formula: {entry:?}");
                    if entry.taxes != 0.0 {
                        Err(anyhow!(
                            "Non-empty taxes in table of type Formel: {}",
                            entry.taxes
                        ))
                    } else if entry.percent != 0.0 {
                        Err(anyhow!(
                            "Non-empty percent in table of type Formel: {}",
                            entry.percent
                        ))
                    } else {
                        if i == 0 && entry.amount != 0.0 {
                            warn!("No entry found for 0 in table of type Formel");
                        }
                        let formula = Formula::try_from(entry.formula.as_str())?;
                        debug!("Parsed formula: {formula:?}");
                        Ok(TableFormelEntry {
                            bracket_start: entry.amount,
                            formula,
                        })
                    }
                })
                .try_collect()?,
        ))
    }
}

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

#[derive(Debug, Clone, Serialize)]
struct TableFormelEntry {
    bracket_start: f64,
    formula: Formula,
}

#[derive(Debug, Clone, Serialize)]
struct TableFreiburg(Vec<TableFreiburgEntry>);

impl TryFrom<&[ScaleEntry]> for TableFreiburg {
    type Error = anyhow::Error;

    fn try_from(table: &[ScaleEntry]) -> Result<Self, Self::Error> {
        Ok(TableFreiburg(
            table
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    if !entry.formula.is_empty() {
                        Err(anyhow!(
                            "Non-empty formula in table of type Freiburg: {:?}",
                            entry.formula
                        ))
                    } else if entry.taxes != 0.0 {
                        Err(anyhow!(
                            "Non-empty taxes in table of type Freiburg: {}",
                            entry.taxes
                        ))
                    } else {
                        if i == 0 && entry.amount != 0.0 {
                            warn!("No entry found for 0 in table of type Freiburg");
                        }
                        Ok(TableFreiburgEntry {
                            bracket_start: entry.amount,
                            tax_rate: entry.percent,
                        })
                    }
                })
                .try_collect()?,
        ))
    }
}

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

#[derive(Debug, Clone, Copy, Serialize)]
struct TableFreiburgEntry {
    bracket_start: f64,
    tax_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
struct TableZuerich(Vec<TableZuerichEntry>);

impl TryFrom<&[ScaleEntry]> for TableZuerich {
    type Error = anyhow::Error;

    fn try_from(table: &[ScaleEntry]) -> Result<Self, Self::Error> {
        Ok(TableZuerich(
            table
                .iter()
                .map(|entry| {
                    if !entry.formula.is_empty() {
                        Err(anyhow!(
                            "Non-empty formula in table of type Zuerich: {:?}",
                            entry.formula
                        ))
                    } else if entry.taxes != 0.0 {
                        Err(anyhow!(
                            "Non-empty taxes in table of type Zuerich: {}",
                            entry.taxes
                        ))
                    } else {
                        let bracket_len = if entry.amount < 10_000_000.0 {
                            entry.amount
                        } else {
                            f64::INFINITY
                        };
                        Ok(TableZuerichEntry {
                            bracket_len,
                            marginal_rate: entry.percent,
                        })
                    }
                })
                .try_collect()?,
        ))
    }
}

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

#[derive(Debug, Clone, Copy, Serialize)]
struct TableZuerichEntry {
    bracket_len: f64,
    marginal_rate: f64,
}
