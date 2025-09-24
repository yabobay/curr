#![allow(non_snake_case)]

use cashkit;
use chrono::{NaiveDateTime, Utc};
use comfy_table::Table;
use levenshtein::levenshtein;
use postcard::{from_bytes, to_allocvec};
use rust_decimal::Decimal;
use rusty_money::{iso, Money};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::panic::catch_unwind;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Clone)]
struct ExchangeRate {
    from: String,
    to: String,
    rate: f64,
    obtention: NaiveDateTime, // always in UTC so it's okay
}

impl ExchangeRate {
    fn new<T: Into<String>>(from: T, to: T, rate: f64) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            rate,
            obtention: Utc::now().naive_utc(),
        }
    }
    fn flip(&self) -> Self {
        Self {
            from: self.to.clone(),
            to: self.from.clone(),
            rate: 1.0 / self.rate,
            obtention: self.obtention,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct CurrencyInformation {
    rates: Vec<ExchangeRate>,
}

impl CurrencyInformation {
    fn new() -> Self {
        Self { rates: vec![] }
    }
    fn add(&mut self, rate: ExchangeRate) {
        self.rates.push(rate.flip());
        self.rates.push(rate);
    }
    fn remove<T: AsRef<str>>(&mut self, from: T, to: T) -> bool {
        for i in 0..(self.rates.len() - 1) {
            if self.rates[i].from == from.as_ref() && self.rates[i].to == to.as_ref() {
                self.rates.swap_remove(i);
                return true;
            }
        }
        return false;
    }
    fn find<T: AsRef<str>>(&mut self, from: T, to: T) -> Option<&ExchangeRate> {
        for i in &self.rates {
            if i.from == from.as_ref() && i.to == to.as_ref() {
                return Some(i);
            }
        }
        None
    }
    fn getRate<T: Into<String>>(&mut self, from: T, to: T) -> Result<f64, CurrErr> {
        let from = &from.into().to_uppercase();
        let to = &to.into().to_uppercase();
        for curr in &[from, to] {
            if !cashkit::code_currency(curr).is_some() {
                return Err(CurrErr::StrangeCurrencies(curr.to_string()));
            }
        }
        if let Some(rate) = self.find(from, to) {
            if (Utc::now().naive_utc() - rate.obtention).num_weeks() < 1 {
                return Ok(rate.rate);
            }
            self.remove(from, to);
        }
        return match catch_unwind(|| cashkit::exchange(from, to, 1.)) {
            Ok(rate) => {
                self.add(ExchangeRate::new(from, to, rate));
                Ok(rate)
            }
            Err(_) => Err(CurrErr::InternetProblem()),
        };
    }
    fn convert(&mut self, from: &String, to: &String, price: f64) -> Result<f64, CurrErr> {
        if from == to {
            return Ok(price);
        }
        match self.getRate(from, to) {
            Ok(rate) => Ok(price * rate),
            Err(e) => Err(e),
        }
    }
}

enum CurrErr {
    StrangeCurrencies(String),
    InternetProblem(),
}

impl From<CurrErr> for String {
    fn from(value: CurrErr) -> Self {
        match value {
            CurrErr::StrangeCurrencies(curr) => {
                let didYouMean = cashkit::active_currencies()
                    .into_iter()
                    .min_by_key(|x| levenshtein(x.code, &curr))
                    .unwrap();
                format!(
                    "{} isn't a real currency! Did you mean {} ({})?",
                    curr, didYouMean.code, didYouMean.name
                )
            }
            CurrErr::InternetProblem() => "Something went wrong with the internet!".to_string(),
        }
    }
}

fn formatCurrency<T: AsRef<str>>(currency: T, amount: f64) -> String {
    match iso::find(&currency.as_ref()) {
        Some(currency) => format!(
            "{}",
            Money::from_decimal(Decimal::from_f64_retain(amount).unwrap(), currency)
        ),
        None => amount.to_string(),
    }
}

#[allow(deprecated)]
fn main() -> Result<(), String> {
    let filename = (match std::env::home_dir() {
        Some(dir) => dir,
        None => std::env::current_dir().unwrap(),
    })
    .join(".curr-cache");

    let mut info: CurrencyInformation = {
        let mut info = None;
        match File::open(&filename) {
            Ok(mut file) => {
                let mut buf = Vec::<u8>::new();
                file.read_to_end(&mut buf).unwrap();
                if buf.len() > 0 {
                    info = Some(from_bytes(&buf).unwrap());
                }
            }
            Err(_) => (),
        };
        info
    }
    .unwrap_or_else(|| CurrencyInformation::new());

    // unfortunately we can't make a Vec of Currencys because the type is private :(
    let mut currencies = Vec::<String>::new();
    let mut prices = Vec::<f64>::new();
    for i in std::env::args().skip(1) {
        match f64::from_str(&i) {
            Ok(i) => prices.push(i),
            Err(_) => {
                let code = i.to_uppercase();
                match cashkit::code_currency(&code) {
                    Some(_) => currencies.push(code),
                    None => return Err(CurrErr::StrangeCurrencies(code).into()),
                }
            }
        }
    }
    if prices.is_empty() {
        prices.push(1.);
    }

    let mut table = Table::new();
    table.set_header(
        (&currencies)
            .into_iter()
            .map(|x| cashkit::code_currency(&x).expect("?").name),
    );
    for p in prices {
        let mut values: Vec<String> = Vec::new();
        for c in &currencies {
            values.push(formatCurrency(c, info.convert(&currencies[0], c, p)?));
        }
        table.add_row(values);
    }
    println!("{table}");

    match File::create(&filename) {
        Ok(mut file) => file.write_all(&to_allocvec(&info).unwrap()).unwrap(),
        Err(e) => return Err(e.to_string()),
    }

    Ok(())
}
