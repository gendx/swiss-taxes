use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt::{self, Display};
use std::marker::PhantomData;
use std::str::FromStr;

// Rates
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rates {
    pub response: Vec<Rate>,
}

#[expect(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "PascalCase")]
pub struct Rate {
    pub location: Location,
    capital_tax_rate_canton: f64,
    capital_tax_rate_church: f64,
    capital_tax_rate_city: f64,
    fortune_rate_canton: f64,
    fortune_rate_christ: f64,
    fortune_rate_city: f64,
    fortune_rate_protestant: f64,
    fortune_rate_roman: f64,
    pub income_rate_canton: f64,
    income_rate_christ: f64,
    income_rate_city: f64,
    income_rate_protestant: f64,
    income_rate_roman: f64,
    profit_tax_rate_canton: f64,
    profit_tax_rate_church: f64,
    profit_tax_rate_city: f64,
}

// Scales
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scales {
    pub response: Vec<Scale>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "PascalCase")]
pub struct Scale {
    pub location: Location,
    #[serde(deserialize_with = "comma_separated")]
    pub group: Vec<Group>,
    pub splitting: f64,
    pub table_type: TableType,
    pub target: Target,
    pub tax_type: TaxType,
    pub table: Vec<ScaleEntry>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "PascalCase")]
pub struct ScaleEntry {
    pub formula: String,
    pub taxes: f64,
    pub percent: f64,
    pub amount: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TableType {
    #[serde(rename = "")]
    Unknown,
    Bund,
    Flattax,
    Formel,
    Freiburg,
    Zuerich,
}

// Deductions
#[expect(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deductions {
    response: Vec<Deduction>,
}

#[expect(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "PascalCase")]
struct Deduction {
    location: Location,
    target: Target,
    tax_type: TaxType,
    table: Vec<DeductionEntry>,
}

#[expect(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "PascalCase")]
struct DeductionEntry {
    minimum: f64,
    maximum: f64,
    #[serde(deserialize_with = "comma_separated")]
    format: Vec<Format>,
    percent: f64,
    amount: f64,
    name: Name,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Format {
    Maximum,
    Minimum,
    Percent,
    Standardized,
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "MAXIMUM" => Ok(Format::Maximum),
            "MINIMUM" => Ok(Format::Minimum),
            "PERCENT" => Ok(Format::Percent),
            "STANDARDIZED" => Ok(Format::Standardized),
            _ => Err(format!("Unknown format: {s}")),
        }
    }
}

// Other deductions
#[expect(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OtherDeductions {
    response: Vec<OtherDeduction>,
}

#[expect(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "PascalCase")]
struct OtherDeduction {
    location: Location,
    #[serde(deserialize_with = "comma_separated")]
    group: Vec<Group>,
    splitting: f64,
    table_type: TableType,
    target: Target,
    tax_type: TaxType,
    table: Vec<ScaleEntry>,
    name: Name,
}

// Common
#[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "PascalCase")]
pub struct Location {
    #[serde(rename = "BfsID")]
    bfs_id: u32,
    bfs_name: String,
    #[serde(rename = "CantonID")]
    pub canton_id: u32,
    pub canton: String,
    city: String,
    #[serde(rename = "TaxLocationID")]
    pub tax_location_id: u32,
    zip_code: String,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "UPPERCASE")]
struct Name {
    id: String,
    de: String,
    en: String,
    fr: String,
    it: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Target {
    Bund,
    Gemeinde,
    Kanton,
    Kirche,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TaxType {
    EinkommensSteuer,
    Erbschaft,
    GewinnSteuer,
    KapitalSteuer,
    VermoegensSteuer,
    VorsorgeSteuer,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Alle,
    LedigAlleine,
    LedigKonkubinat,
    LedigMitKinder,
    LedigOhneKinder,
    TypGeschwisterGeschwister,
    TypGeschwisterStiefgeschwister,
    TypGrosselternGrosseltern,
    TypGrosselternPflegegrosseltern,
    TypGrosselternStiefgrosseltern,
    TypGrosselternUrgrosseltern,
    TypEhepartnerEhepartner,
    TypElternEltern,
    TypElternPflegeeltern,
    TypElternStiefeltern,
    TypKinderKinder,
    TypKinderNachkommenkinder,
    TypKinderNachkommenpflegekinder,
    TypKinderNachkommenstiefkinder,
    TypKinderPatenkinder,
    TypKinderPflegekinder,
    TypKinderStiefkinder,
    TypKinderVollwaisen,
    TypOnkeltantenCousin,
    TypOnkeltantenGrossneffen,
    TypOnkeltantenGrossonkel,
    TypOnkeltantenNachkommencousin,
    TypOnkeltantenNeffen,
    TypOnkeltantenOnkel,
    TypOnkeltantenUrgrossneffen,
    TypPartnerLebenspartner,
    TypPartnerLebenspartnerMitKind,
    TypPartnerVerlobter,
    TypUebrigeAngestellte,
    TypUebrigeBeschraenkt,
    TypUebrigeDauernBeduerftigt,
    TypUebrigePersonenvereinigungen,
    TypUebrigeSchwiegereltern,
    TypUebrigeSchwiegersohn,
    TypUebrigeStiftungen,
    TypUebrigeUebrige,
    TypUebrigeUnehelichekinder,
    TypUebrigeVerschwaegerte,
    Verheiratet,
}

impl FromStr for Group {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ALLE" => Ok(Group::Alle),
            "LEDIG_ALLEINE" => Ok(Group::LedigAlleine),
            "LEDIG_KONKUBINAT" => Ok(Group::LedigKonkubinat),
            "LEDIG_OHNE_KINDER" => Ok(Group::LedigOhneKinder),
            "LEDIG_MIT_KINDER" => Ok(Group::LedigMitKinder),
            "TYP_GESCHWISTER_GESCHWISTER" => Ok(Group::TypGeschwisterGeschwister),
            "TYP_GESCHWISTER_STIEFGESCHWISTER" => Ok(Group::TypGeschwisterStiefgeschwister),
            "TYP_GROSSELTERN_GROSSELTERN" => Ok(Group::TypGrosselternGrosseltern),
            "TYP_GROSSELTERN_PFLEGEGROSSELTERN" => Ok(Group::TypGrosselternPflegegrosseltern),
            "TYP_GROSSELTERN_STIEFGROSSELTERN" => Ok(Group::TypGrosselternStiefgrosseltern),
            "TYP_GROSSELTERN_URGROSSELTERN" => Ok(Group::TypGrosselternUrgrosseltern),
            "TYP_EHEPARTNER_EHEPARTNER" => Ok(Group::TypEhepartnerEhepartner),
            "TYP_ELTERN_ELTERN" => Ok(Group::TypElternEltern),
            "TYP_ELTERN_PFLEGEELTERN" => Ok(Group::TypElternPflegeeltern),
            "TYP_ELTERN_STIEFELTERN" => Ok(Group::TypElternStiefeltern),
            "TYP_KINDER_KINDER" => Ok(Group::TypKinderKinder),
            "TYP_KINDER_NACHKOMMENKINDER" => Ok(Group::TypKinderNachkommenkinder),
            "TYP_KINDER_NACHKOMMENPFLEGEKINDER" => Ok(Group::TypKinderNachkommenpflegekinder),
            "TYP_KINDER_NACHKOMMENSTIEFKINDER" => Ok(Group::TypKinderNachkommenstiefkinder),
            "TYP_KINDER_PATENKINDER" => Ok(Group::TypKinderPatenkinder),
            "TYP_KINDER_PFLEGEKINDER" => Ok(Group::TypKinderPflegekinder),
            "TYP_KINDER_STIEFKINDER" => Ok(Group::TypKinderStiefkinder),
            "TYP_KINDER_VOLLWAISEN" => Ok(Group::TypKinderVollwaisen),
            "TYP_ONKELTANTEN_COUSIN" => Ok(Group::TypOnkeltantenCousin),
            "TYP_ONKELTANTEN_GROSSNEFFEN" => Ok(Group::TypOnkeltantenGrossneffen),
            "TYP_ONKELTANTEN_GROSSONKEL" => Ok(Group::TypOnkeltantenGrossonkel),
            "TYP_ONKELTANTEN_NACHKOMMENCOUSIN" => Ok(Group::TypOnkeltantenNachkommencousin),
            "TYP_ONKELTANTEN_NEFFEN" => Ok(Group::TypOnkeltantenNeffen),
            "TYP_ONKELTANTEN_ONKEL" => Ok(Group::TypOnkeltantenOnkel),
            "TYP_ONKELTANTEN_URGROSSNEFFEN" => Ok(Group::TypOnkeltantenUrgrossneffen),
            "TYP_PARTNER_LEBENSPARTNER" => Ok(Group::TypPartnerLebenspartner),
            "TYP_PARTNER_LEBENSPARTNER_MIT_KIND" => Ok(Group::TypPartnerLebenspartnerMitKind),
            "TYP_PARTNER_VERLOBTER" => Ok(Group::TypPartnerVerlobter),
            "TYP_UEBRIGE_ANGESTELLTE" => Ok(Group::TypUebrigeAngestellte),
            "TYP_UEBRIGE_BESCHRAENKT" => Ok(Group::TypUebrigeBeschraenkt),
            "TYP_UEBRIGE_DAUERND_BEDUERFTIGT" => Ok(Group::TypUebrigeDauernBeduerftigt),
            "TYP_UEBRIGE_PERSONENVEREINIGUNGEN" => Ok(Group::TypUebrigePersonenvereinigungen),
            "TYP_UEBRIGE_SCHWIEGERELTERN" => Ok(Group::TypUebrigeSchwiegereltern),
            "TYP_UEBRIGE_SCHWIEGERSOHN" => Ok(Group::TypUebrigeSchwiegersohn),
            "TYP_UEBRIGE_STIFTUNGEN" => Ok(Group::TypUebrigeStiftungen),
            "TYP_UEBRIGE_UEBRIGE" => Ok(Group::TypUebrigeUebrige),
            "TYP_UEBRIGE_UNEHELICHEKINDER" => Ok(Group::TypUebrigeUnehelichekinder),
            "TYP_UEBRIGE_VERSCHWAEGERTE" => Ok(Group::TypUebrigeVerschwaegerte),
            "VERHEIRATET" => Ok(Group::Verheiratet),
            _ => Err(format!("Unknown group: {s}")),
        }
    }
}

// Helpers
fn comma_separated<'de, V, T, D>(deserializer: D) -> Result<V, D::Error>
where
    V: FromIterator<T>,
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    struct CommaSeparated<V, T>(PhantomData<V>, PhantomData<T>);

    impl<'de, V, T> Visitor<'de> for CommaSeparated<V, T>
    where
        V: FromIterator<T>,
        T: FromStr,
        T::Err: Display,
    {
        type Value = V;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string containing comma-separated elements")
        }

        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let iter = s.split(",").filter_map(|x| {
                if x.is_empty() {
                    None
                } else {
                    Some(FromStr::from_str(x))
                }
            });
            Result::from_iter(iter).map_err(de::Error::custom)
        }
    }

    let visitor = CommaSeparated(PhantomData, PhantomData);
    deserializer.deserialize_str(visitor)
}
