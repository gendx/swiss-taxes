use anyhow::anyhow;
use log::warn;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::multispace0;
use nom::combinator::{map, map_res};
use nom::multi::many;
use nom::number::complete::recognize_float;
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};
use ordered_float::OrderedFloat;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum Formula {
    Input,
    Const(OrderedFloat<f64>),
    Log(Box<Formula>),
    Add(Box<Formula>, Box<Formula>),
    Sub(Box<Formula>, Box<Formula>),
    Mul(Box<Formula>, Box<Formula>),
    Div(Box<Formula>, Box<Formula>),
}

#[cfg(test)]
impl Formula {
    fn log(f: Formula) -> Self {
        Self::Log(Box::new(f))
    }

    fn add(f: Formula, g: Formula) -> Self {
        Self::Add(Box::new(f), Box::new(g))
    }

    fn sub(f: Formula, g: Formula) -> Self {
        Self::Sub(Box::new(f), Box::new(g))
    }

    fn mul(f: Formula, g: Formula) -> Self {
        Self::Mul(Box::new(f), Box::new(g))
    }
}

impl Formula {
    fn constant(x: f64) -> Self {
        Self::Const(OrderedFloat(x))
    }

    pub fn eval(&self, x: f64) -> f64 {
        match self {
            Formula::Input => x,
            Formula::Const(c) => **c,
            Formula::Log(f) => f.eval(x).ln(),
            Formula::Add(f, g) => f.eval(x) + g.eval(x),
            Formula::Sub(f, g) => f.eval(x) - g.eval(x),
            Formula::Mul(f, g) => f.eval(x) * g.eval(x),
            Formula::Div(f, g) => f.eval(x) / g.eval(x),
        }
    }
}

impl TryFrom<&str> for Formula {
    type Error = anyhow::Error;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        if text.is_empty() {
            Ok(Formula::constant(0.0))
        } else {
            match expr(text) {
                Ok((remainder, formula)) => {
                    if remainder.is_empty() {
                        Ok(formula)
                    } else {
                        warn!("Incomplete parsing, formula: {formula:?}, remainder: {remainder}");
                        Err(anyhow!(
                            "Incomplete parsing, formula: {formula:?}, remainder: {remainder}"
                        ))
                    }
                }
                Err(e) => {
                    warn!("Failed to parse: {e:?}");
                    Err(anyhow!("Failed to parse: {e:?}"))
                }
            }
        }
    }
}

enum Operation {
    Add,
    Sub,
    Mul,
    Div,
}

fn parens(i: &str) -> IResult<&str, Formula> {
    delimited(
        multispace0,
        delimited(tag("("), expr, tag(")")),
        multispace0,
    )
    .parse(i)
}

fn expr(i: &str) -> IResult<&str, Formula> {
    let (i, initial) = term(i)?;
    let (i, remainder) = many(
        0..,
        alt((
            |i| {
                let (i, add) = preceded(tag("+"), term).parse(i)?;
                Ok((i, (Operation::Add, add)))
            },
            |i| {
                let (i, sub) = preceded(tag("-"), term).parse(i)?;
                Ok((i, (Operation::Sub, sub)))
            },
        )),
    )
    .parse(i)?;

    Ok((i, fold_exprs(initial, remainder)))
}

fn term(i: &str) -> IResult<&str, Formula> {
    let (i, initial) = factor(i)?;
    let (i, remainder) = many(
        0..,
        alt((
            |i| {
                let (i, mul) = preceded(tag("*"), term).parse(i)?;
                Ok((i, (Operation::Mul, mul)))
            },
            |i| {
                let (i, div) = preceded(tag("/"), term).parse(i)?;
                Ok((i, (Operation::Div, div)))
            },
        )),
    )
    .parse(i)?;

    Ok((i, fold_exprs(initial, remainder)))
}

fn factor(i: &str) -> IResult<&str, Formula> {
    alt((
        parens,
        map(
            map_res(
                delimited(multispace0, recognize_float, multispace0),
                |s: &str| s.parse::<f64>().map(OrderedFloat),
            ),
            Formula::Const,
        ),
        map(delimited(multispace0, tag("$wert$"), multispace0), |_| {
            Formula::Input
        }),
        map(
            preceded(delimited(multispace0, tag("log"), multispace0), factor),
            |f| Formula::Log(Box::new(f)),
        ),
    ))
    .parse(i)
}

fn fold_exprs(initial: Formula, remainder: Vec<(Operation, Formula)>) -> Formula {
    remainder.into_iter().fold(initial, |acc, pair| {
        let (operation, expr) = pair;
        match operation {
            Operation::Add => Formula::Add(Box::new(acc), Box::new(expr)),
            Operation::Sub => Formula::Sub(Box::new(acc), Box::new(expr)),
            Operation::Mul => Formula::Mul(Box::new(acc), Box::new(expr)),
            Operation::Div => Formula::Div(Box::new(acc), Box::new(expr)),
        }
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_formula() {
        assert_eq!(
            Formula::try_from(
                "-0.827429* $wert$ + 0.089718* $wert$ * (log $wert$ - 1) + 829.418770"
            )
            .unwrap(),
            Formula::add(
                Formula::add(
                    Formula::mul(Formula::constant(-0.827429), Formula::Input),
                    Formula::mul(
                        Formula::constant(0.089718),
                        Formula::mul(
                            Formula::Input,
                            Formula::sub(Formula::log(Formula::Input), Formula::constant(1.0))
                        )
                    )
                ),
                Formula::constant(829.41877)
            )
        )
    }

    #[test]
    fn parse_input() {
        assert_eq!(Formula::try_from("$wert$").unwrap(), Formula::Input);
        assert_eq!(Formula::try_from("  $wert$").unwrap(), Formula::Input);
        assert_eq!(Formula::try_from("$wert$  ").unwrap(), Formula::Input);
        assert_eq!(Formula::try_from(" $wert$  ").unwrap(), Formula::Input);
    }

    #[test]
    fn parse_const() {
        assert_eq!(Formula::try_from("100").unwrap(), Formula::constant(100.0));
        assert_eq!(
            Formula::try_from("12.34").unwrap(),
            Formula::constant(12.34)
        );
        assert_eq!(
            Formula::try_from("-42.42").unwrap(),
            Formula::constant(-42.42)
        );
    }
}
