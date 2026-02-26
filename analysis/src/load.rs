use crate::schema::{Group, Rates, Scales, Target, TaxType};
use crate::table::{EvalPolicy, Table};
use anyhow::{Result, anyhow};
use log::{debug, trace};
use serde::Serialize;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufReader, BufWriter};

#[derive(Serialize)]
pub struct Database(BTreeMap<u32, Year>);

impl Database {
    pub fn new(years: impl Iterator<Item = u32>) -> Result<Self> {
        let db = years
            .map(|year| -> Result<_> { Ok((year, Year::new(year)?)) })
            .try_collect()?;
        Ok(Database(db))
    }

    pub fn serialize(&self) -> Result<()> {
        let file = File::create_new("data/tables.db")?;
        postcard::to_io(self, BufWriter::new(file))?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct Year(BTreeMap<String, CantonalBase>);

impl Year {
    fn new(year: u32) -> Result<Self> {
        let rates = get_cantonal_rates(year)?;
        let scales = get_cantonal_scales(year)?;

        let mut map = BTreeMap::new();
        for (canton, scale) in scales {
            if canton == "VS" {
                continue;
            }
            let rate = rates[&canton];
            map.insert(canton, CantonalBase { rate, scale });
        }
        Ok(Year(map))
    }
}

#[derive(Serialize)]
struct CantonalBase {
    rate: f64,
    scale: CantonalScale,
}

pub fn canton_policy(canton: &str) -> Result<EvalPolicy> {
    match canton {
        "BL" | "GE" | "GR" | "SO" => Ok(EvalPolicy::Raw),
        "UR" => Ok(EvalPolicy::NoSplitRaw),
        "AG" => Ok(EvalPolicy::Round100),
        "AI" | "FR" | "GL" | "NE" | "NW" | "SG" | "SH" | "SZ" | "TG" | "VD" => {
            Ok(EvalPolicy::DoubleRound100)
        }
        "AR" | "BE" | "BS" | "JU" | "LU" | "OW" | "TI" | "ZG" | "ZH" | "CH" => {
            Ok(EvalPolicy::NoSplitRound100)
        }
        "VS" => Ok(EvalPolicy::Valais),
        x => Err(anyhow!("Unknown canton: {x}")),
    }
}

pub fn get_cantonal_rates(year: u32) -> Result<HashMap<String, f64>> {
    debug!("Loading cantonal rates for {year}");
    let rates: Rates = serde_json::from_reader(BufReader::new(File::open(format!(
        "data/rates-{year}.json"
    ))?))?;

    let mut cantonal_rates: HashMap<String, f64> = HashMap::new();
    for rate in &rates.response {
        trace!("Rate: {:?}", rate);
        let mut income_rate_canton = rate.income_rate_canton;
        if rate.location.canton == "GE" {
            // See https://www.getax.ch/support/guide/declaration2024/Impotsurlerevenubaremesetcalculs.html
            income_rate_canton *= 0.88;
            income_rate_canton += 1.0;
        } else if rate.location.canton == "VD" && year >= 2024 {
            // See https://www.vd.ch/actualites/communiques-de-presse-de-letat-de-vaud/detail/communique/le-conseil-detat-respecte-ses-engagements-et-detaille-sa-feuille-de-route-fiscale-1695286975
            income_rate_canton *= 0.965;
        }
        // TODO: VS https://fbk-conseils.ch/impot-cantonaux-en-valais/

        match cantonal_rates.entry(rate.location.canton.clone()) {
            Entry::Occupied(entry) => {
                if *entry.get() != income_rate_canton {
                    return Err(anyhow!(
                        "Inconsistent cantonal income rate in {}: {} != {}",
                        rate.location.canton,
                        entry.get(),
                        income_rate_canton
                    ));
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(income_rate_canton);
            }
        }
    }

    cantonal_rates.insert("CH".into(), 100.0);

    Ok(cantonal_rates)
}

#[derive(Serialize)]
pub struct CantonalScale {
    pub splitting: f64,
    pub single: Table,
    pub married: Table,
}

pub fn get_cantonal_scales(year: u32) -> Result<HashMap<String, CantonalScale>> {
    let scales: Scales = serde_json::from_reader(BufReader::new(File::open(format!(
        "data/scales-{year}.json"
    ))?))?;

    let mut cantonal_scales_single = HashMap::new();
    let mut cantonal_scales_married = HashMap::new();
    scales
        .response
        .iter()
        .filter(|scale| {
            scale.tax_type == TaxType::EinkommensSteuer && scale.target == Target::Kanton
        })
        .try_for_each(|scale| -> Result<()> {
            trace!("Cantonal scale: {scale:?}");
            let single = is_single(&scale.group);
            let married = is_married(&scale.group);
            let policy = canton_policy(&scale.location.canton)?;
            if (single || married)
                && let Ok(table) = Table::try_from(scale, policy)
            {
                if single {
                    cantonal_scales_single.insert(scale.location.canton.clone(), table.clone());
                }
                if married {
                    cantonal_scales_married
                        .insert(scale.location.canton.clone(), (scale.splitting, table));
                }
            }

            Ok(())
        })?;

    // Federal scale.
    scales
        .response
        .iter()
        .filter(|scale| {
            scale.tax_type == TaxType::EinkommensSteuer
                && scale.target == Target::Bund
                && scale.location.canton_id == 1
        })
        .try_for_each(|scale| -> Result<()> {
            trace!("Federal scale: {scale:?}");
            let single = is_single(&scale.group);
            let married = is_married(&scale.group);
            let policy = canton_policy("CH")?;
            if (single || married)
                && let Ok(table) = Table::try_from(scale, policy)
            {
                if single {
                    cantonal_scales_single.insert("CH".into(), table.clone());
                }
                if married {
                    cantonal_scales_married.insert("CH".into(), (scale.splitting, table));
                }
            }

            Ok(())
        })?;

    let mut cantonal_scales = HashMap::new();
    for (canton, table_single) in cantonal_scales_single {
        if let Some((splitting, table_married)) = cantonal_scales_married.remove(&canton) {
            cantonal_scales.insert(
                canton,
                CantonalScale {
                    splitting,
                    single: table_single,
                    married: table_married,
                },
            );
        }
    }

    Ok(cantonal_scales)
}

pub fn is_single(group: &[Group]) -> bool {
    group
        .iter()
        .any(|&x| x == Group::Alle || x == Group::LedigAlleine)
        && !group.contains(&Group::Verheiratet)
}

pub fn is_married(group: &[Group]) -> bool {
    group
        .iter()
        .any(|&x| x == Group::Alle || x == Group::Verheiratet)
}
