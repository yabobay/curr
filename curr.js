import googleCurrencyScraper from "google-currency-scraper";
import AsciiTable from "ascii-table";
import fs from "node:fs/promises";
import path from "node:path";

class Curr {
    static rates;
    static needToSave = false;
    static ratesPath = path.join(import.meta.dirname, "rates.json");
    static async load() {
        try {
            this.rates = JSON.parse(await fs.readFile(this.ratesPath));
        } catch (SyntaxError) {
            this.rates = {};
        }
    }

    static async convert(amount, a, b) {
        const now = Date.now();
        /* get exchange rate from google if it doesn't exist or if
           cache is older than 12 hours */
        if (!this.rates[[a, b]] || now - this.rates[[a, b]].date > 43200) {
            const result = await googleCurrencyScraper({ from: a, to: b });
            this.rates[[a, b]] = { date: now, rate: result.rate };
            this.rates[[b, a]] = { date: now, rate: 1 / result.rate };
            this.needToSave = true;
        }
        return parseFloat((amount * this.rates[[a, b]].rate).toFixed(2));
    }

    static async save() {
        if (this.needToSave)
            await fs.writeFile(this.ratesPath, JSON.stringify(this.rates));
    }
}

async function doAPrettyTable(argv) {
    let prices = [];
    let currencies = [];

    argv.forEach((x) => {
        if (Number(x)) prices.push(x);
        else currencies.push(x.toUpperCase());
    });

    if (!prices.length) prices.push(1);

    await Curr.load();

    var table = new AsciiTable();

    table.setHeading(currencies);

    for (let p = 0; p < prices.length; p++) {
        let values = [prices[p]];
        for (let c = 1; c < currencies.length; c++)
            values.push(Curr.convert(prices[p], currencies[0], currencies[c]));
        table.addRow(await Promise.all(values));
    }

    Curr.save();

    return table.toString();
}

console.log(await doAPrettyTable(process.argv.slice(2)));
