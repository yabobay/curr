#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

use comfy_table::Table;
use std::collections::BTreeSet;
use std::fs::File;
use std::panic::{catch_unwind, set_hook};
use std::str::FromStr;

use cashkit;
use rust_decimal::Decimal;
use rusty_money::{iso, iso::Currency, FormattableCurrency, Money};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
struct ExchangeRate {
    from: String,
    to: String,
    rate: f64,
}

impl ExchangeRate {
    fn new<T: Into<String>>(from: T, to: T, rate: f64) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            rate,
        }
    }
    fn flip(&self) -> Self {
        Self {
            from: self.to.clone(),
            to: self.from.clone(),
            rate: 1.0 / self.rate,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct CurrencyInformation {
    rates: Vec<ExchangeRate>,
}

enum CurrErr {
    StrangeCurrencies(String, String),
    InternetProblem(),
}

impl From<CurrErr> for String {
    fn from(value: CurrErr) -> Self {
        match value {
            CurrErr::StrangeCurrencies(a, b) => format!("Either {a} or {b} isn't a real currency!"),
            CurrErr::InternetProblem() => "Something went wrong with the internet!".to_string(),
        }
    }
}

impl CurrencyInformation {
    fn new() -> Self {
        Self { rates: vec![] }
    }
    fn add(&mut self, rate: ExchangeRate) {
        self.rates.push(rate.flip());
        self.rates.push(rate);
    }
    fn getRate<T: Into<String>>(&mut self, from: T, to: T) -> Result<f64, CurrErr> {
        let from = &from.into().to_uppercase();
        let to = &to.into().to_uppercase();
        if !(cashkit::code_currency(from).is_some() && cashkit::code_currency(to).is_some()) {
            return Err(CurrErr::StrangeCurrencies(from.to_string(), to.to_string()));
        }
        // TODO: find rate in rates
        match catch_unwind(|| cashkit::exchange(from, to, 1.)) {
            Ok(rate) => {
                self.add(ExchangeRate::new(from, to, rate));
                Ok(rate)
            }
            Err(_) => Err(CurrErr::InternetProblem()),
        }
    }
    fn convert<T: Into<String>>(&mut self, from: T, to: T, price: f64) -> Result<f64, CurrErr> {
        match self.getRate(from, to) {
            Ok(rate) => Ok(price * rate),
            Err(e) => Err(e),
        }
    }
}

#[allow(unused_must_use)]
impl std::fmt::Display for CurrencyInformation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "Elements: {}", self.rates.len());
        Ok(())
    }
}

fn formatCurrency<T: Into<String> + ?Sized>(currency: T, amount: f64) -> String {
    match iso::find(&currency.into()) {
        Some(currency) => {
            format!(
                "{}",
                Money::from_decimal(Decimal::from_f64_retain(amount).unwrap(), currency)
            )
        }
        None => amount.to_string(),
    }
}

fn slurpFile(filename: String) -> Option<String> {
    let mut file = File::open(filename).expect("");
    let mut contents = String::new();
    match std::io::Read::read_to_string(&mut file, &mut contents) {
        Ok(_) => Some(contents),
        Err(_) => None,
    }
}

fn main() -> Result<(), String> {
    let mut info = CurrencyInformation::new();
    let mut currencies: Vec<String> = Vec::new();
    let mut prices: Vec<f64> = Vec::new();
    for i in std::env::args().skip(1) {
        match f64::from_str(&i) {
            Ok(i) => prices.push(i),
            Err(_) => currencies.push(i),
        }
    }
    currencies = currencies.into_iter().map(|x| x.to_uppercase()).collect();
    if prices.is_empty() {
        prices.push(1.);
    }
    let mut table = Table::new();
    table.set_header(&currencies);
    for p in prices {
        let mut values: Vec<String> = Vec::new();
        for c in &currencies {
            values.push(formatCurrency(c, info.convert(&currencies[0], c, p)?));
        }
        table.add_row(values);
    }
    println!("{table}");
    Ok(())
}
