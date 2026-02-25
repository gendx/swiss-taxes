use crate::load::{get_cantonal_rates, get_cantonal_scales};
use crate::schema::{Location, Rates};
use anyhow::Result;
use log::{debug, info, trace, warn};
use rand::RngExt;
use rand::seq::SliceRandom;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::ops::{AddAssign, Deref};
use tokio::runtime::Runtime;

pub fn check_all_tests(years: impl IntoIterator<Item = u32>) -> Result<()> {
    let mut cantons = BTreeSet::new();
    let mut by_canton: HashMap<(String, Relationship), Matches> = HashMap::new();
    let mut total = Matches::default();
    let mut count_tests = 0;
    for year in years {
        let (num_tests, results) = check_tests(year)?;
        count_tests += num_tests;

        let mut by_year = Matches::default();
        for (key, test_result) in results {
            cantons.insert(key.0.clone());
            let matches = test_result.map_or_default(|r| r.matches());
            *by_canton.entry(key).or_default() += matches;
            by_year += matches;
        }

        total += by_year;
        info!("Matches in {year}: {by_year:?}");
    }

    for relationship in [Relationship::Single, Relationship::Married] {
        for canton in &cantons {
            let matches = by_canton[&(canton.clone(), relationship)];
            info!("Matches in ({canton}, {relationship:?}): {matches:?}");
        }
    }

    info!("Total matches: {total:?} / {count_tests}");

    Ok(())
}

pub struct TestResult {
    expected: Evaluation,
    actual: Evaluation,
}

impl TestResult {
    fn check(&self, year: u32, canton: &str, relationship: Relationship) {
        if self.expected.income_simple_tax_canton != self.actual.income_simple_tax_canton {
            warn!(
                "[{canton}, {year}, {relationship:?}] Mismatch for income_simple_tax_canton: expected {} got {}",
                self.expected.income_simple_tax_canton, self.actual.income_simple_tax_canton
            );
        }
        if self.expected.income_simple_tax_city != self.actual.income_simple_tax_city {
            warn!(
                "[{canton}, {year}, {relationship:?}] Mismatch for income_simple_tax_city: expected {} got {}",
                self.expected.income_simple_tax_city, self.actual.income_simple_tax_city
            );
        }
        if self.expected.income_tax_canton != self.actual.income_tax_canton {
            warn!(
                "[{canton}, {year}, {relationship:?}] Mismatch for income_tax_canton: expected {} got {}",
                self.expected.income_tax_canton, self.actual.income_tax_canton
            );
        }
    }

    fn matches(&self) -> Matches {
        Matches {
            income_simple_tax_canton: if self.expected.income_simple_tax_canton
                == self.actual.income_simple_tax_canton
            {
                1
            } else {
                0
            },
            income_simple_tax_city: if self.expected.income_simple_tax_city
                == self.actual.income_simple_tax_city
            {
                1
            } else {
                0
            },
            income_tax_canton: if self.expected.income_tax_canton == self.actual.income_tax_canton {
                1
            } else {
                0
            },
        }
    }
}

pub struct Evaluation {
    income_simple_tax_canton: f64,
    income_simple_tax_city: f64,
    income_tax_canton: f64,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Matches {
    income_simple_tax_canton: usize,
    income_simple_tax_city: usize,
    income_tax_canton: usize,
}

impl AddAssign for Matches {
    fn add_assign(&mut self, other: Self) {
        self.income_simple_tax_canton += other.income_simple_tax_canton;
        self.income_simple_tax_city += other.income_simple_tax_city;
        self.income_tax_canton += other.income_tax_canton;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Relationship {
    Single,
    Married,
}

#[expect(clippy::type_complexity)]
pub fn check_tests(
    year: u32,
) -> Result<(usize, HashMap<(String, Relationship), Option<TestResult>>)> {
    info!("Checking examples for {year}");
    let tests: TestSuite = serde_json::from_reader(BufReader::new(File::open(format!(
        "data/tests-{year}.json"
    ))?))?;
    let num_tests = tests.0.len();
    debug!("Loaded {num_tests} tests");

    debug!("Loading cantonal scales");
    let cantonal_scales = get_cantonal_scales(year)?;

    debug!("Loading cantonal rates");
    let cantonal_rates = get_cantonal_rates(year)?;

    let mut results = HashMap::new();
    for test in tests.0 {
        let (request, response) = (test.request, test.response.response);
        let canton = response.location.canton;
        let relationship = match request.relationship {
            1 => Relationship::Single,
            2 => Relationship::Married,
            x => panic!("Unknown relationship type: {x}"),
        };

        debug!("Checking {canton}");
        if let (Some(canton_scale), Some(canton_rate)) =
            (cantonal_scales.get(&canton), cantonal_rates.get(&canton))
        {
            let table = match relationship {
                Relationship::Single => &canton_scale.single,
                Relationship::Married => &canton_scale.married,
            };

            let income_simple_tax_canton = match relationship {
                Relationship::Single => table.eval(request.taxable_income_canton.into()),
                Relationship::Married => {
                    table.eval_split(request.taxable_income_canton.into(), canton_scale.splitting)
                }
            };
            let income_tax_canton = income_simple_tax_canton * canton_rate / 100.0;
            // TODO: not in VS
            let income_simple_tax_city = income_simple_tax_canton;

            let expected = Evaluation {
                income_simple_tax_canton: response.income_simple_tax_canton,
                income_simple_tax_city: response.income_simple_tax_city,
                income_tax_canton: response.income_tax_canton,
            };
            let actual = Evaluation {
                income_simple_tax_canton: income_simple_tax_canton.round(),
                income_simple_tax_city: income_simple_tax_city.round(),
                income_tax_canton: income_tax_canton.round(),
            };
            let test_result = TestResult { expected, actual };
            test_result.check(year, &canton, relationship);

            results.insert((canton, relationship), Some(test_result));
        } else {
            results.insert((canton, relationship), None);
        }
    }

    Ok((num_tests, results))
}

pub fn fetch_examples(years: impl Iterator<Item = u32>) -> Result<()> {
    let rt = Runtime::new()?;

    rt.block_on(async {
        let client = Client::new();
        for year in years {
            if let Err(e) = fetch_examples_impl(&client, year).await {
                warn!("Failed to fetch examples for {year}: {e:?}");
            }
        }
        Ok(())
    })
}

async fn fetch_examples_impl(client: &Client, year: u32) -> Result<()> {
    info!("Making test cases for {year}");
    fs::create_dir_all("data")?;

    let path = format!("data/tests-{year}.json");
    let file = File::create_new(&path)?;
    debug!("Created new file: {path:?}");

    let examples = make_examples(year)?;

    let mut tests = Vec::new();
    for request in examples {
        trace!("Evaluating {request:?}");
        match fetch_calculation(client, &request).await {
            Ok(response) => {
                tests.push(Value::Object(
                    [
                        ("request".into(), request.to_json()),
                        ("response".into(), response),
                    ]
                    .into_iter()
                    .collect(),
                ));
            }
            Err(e) => {
                warn!("Failed to fetch calculation for {request:?}: {e:?}");
            }
        }
    }

    debug!("Serializing tests for {year}");
    let json = Value::Array(tests);
    serde_json::to_writer(BufWriter::new(file), &json)?;

    Ok(())
}

fn make_examples(year: u32) -> Result<Vec<Request>> {
    debug!("Making examples for {year}");
    let rates: Rates = serde_json::from_reader(BufReader::new(File::open(format!(
        "data/rates-{year}.json"
    ))?))?;

    let mut locations: HashMap<String, Vec<&Location>> = HashMap::new();
    for rate in &rates.response {
        locations
            .entry(rate.location.canton.clone())
            .or_default()
            .push(&rate.location);
    }

    let mut requests = Vec::new();
    let mut rng = rand::rng();
    for (canton, mut locations) in locations.into_iter() {
        locations.partial_shuffle(&mut rng, 2);

        trace!("- Canton: {canton}");
        for i in 0..2 {
            let location = locations[i % locations.len()];
            trace!("  [{i}] {location:?}");

            let taxable_fortune = rng.random_range(500_000..2_000_000);
            let taxable_income_canton = rng.random_range(50_000..200_000);
            let taxable_income_fed = rng.random_range(50_000..200_000);

            if i == 0 {
                requests.push(Request::make_single(
                    taxable_fortune,
                    taxable_income_canton,
                    taxable_income_fed,
                    location.tax_location_id,
                    year,
                ));
            } else {
                requests.push(Request::make_married(
                    taxable_fortune,
                    taxable_income_canton,
                    taxable_income_fed,
                    location.tax_location_id,
                    year,
                ));
            }
        }
    }
    Ok(requests)
}

async fn fetch_calculation(client: &Client, request: &Request) -> Result<serde_json::Value> {
    const URL: &str = "https://swisstaxcalculator.estv.admin.ch/delegate/ost-integration/v1/lg-proxy/operation/c3b67379_ESTV/API_calculateSimpleTaxes";

    let res = client.post(URL).json(request).send().await?;
    trace!("Status: {:?}", res.status());

    let bytes = res.bytes().await?;
    trace!("Received {} bytes", bytes.len());

    trace!("Parsing as JSON");
    let json = serde_json::from_slice(bytes.deref())?;

    Ok(json)
}

#[derive(Debug, Deserialize)]
struct TestSuite(Vec<Test>);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Test {
    request: Request,
    response: Response,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    response: Example,
}

#[expect(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "PascalCase")]
struct Example {
    location: Location,
    fortune_simple_tax_canton: f64,
    fortune_simple_tax_city: f64,
    fortune_tax_canton: f64,
    fortune_tax_church: f64,
    fortune_tax_city: f64,
    income_simple_tax_canton: f64,
    income_simple_tax_city: f64,
    income_simple_tax_fed: f64,
    income_tax_canton: f64,
    income_tax_church: f64,
    income_tax_city: f64,
    income_tax_fed: f64,
    personal_tax: f64,
    tax_credit: f64,
    total_net_tax: f64,
    total_tax: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "PascalCase")]
struct Request {
    children: Vec<Child>,
    confession1: u32,
    confession2: u32,
    relationship: u32,
    taxable_fortune: u32,
    taxable_income_canton: u32,
    taxable_income_fed: u32,
    #[serde(rename = "TaxLocationID")]
    tax_location_id: u32,
    tax_year: u32,
}

impl Request {
    fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap()
    }

    fn make_single(
        taxable_fortune: u32,
        taxable_income_canton: u32,
        taxable_income_fed: u32,
        tax_location_id: u32,
        tax_year: u32,
    ) -> Self {
        Self {
            children: vec![],
            confession1: 5,
            confession2: 0,
            relationship: 1,
            taxable_fortune,
            taxable_income_canton,
            taxable_income_fed,
            tax_location_id,
            tax_year,
        }
    }

    fn make_married(
        taxable_fortune: u32,
        taxable_income_canton: u32,
        taxable_income_fed: u32,
        tax_location_id: u32,
        tax_year: u32,
    ) -> Self {
        Self {
            children: vec![],
            confession1: 5,
            confession2: 5,
            relationship: 2,
            taxable_fortune,
            taxable_income_canton,
            taxable_income_fed,
            tax_location_id,
            tax_year,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Child;
