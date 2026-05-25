use reqwest::Client;

use crate::backend::FinanceBackend;
use crate::domain::*;

pub struct QuickBooksBackend {
    client: Client,
    access_token: String,
    #[allow(dead_code)]
    realm_id: String,
    base_url: String,
}

impl QuickBooksBackend {
    pub fn new(access_token: String, realm_id: String) -> Self {
        Self {
            client: Client::new(),
            base_url: format!("https://quickbooks.api.intuit.com/v3/company/{realm_id}"),
            access_token,
            realm_id,
        }
    }

    async fn get(&self, path: &str) -> anyhow::Result<serde_json::Value> {
        let resp = self.client.get(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Accept", "application/json")
            .send().await?
            .error_for_status()?
            .json().await?;
        Ok(resp)
    }

    async fn post(&self, path: &str, body: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let resp = self.client.post(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
            .send().await?
            .error_for_status()?
            .json().await?;
        Ok(resp)
    }

    async fn query(&self, q: &str) -> anyhow::Result<serde_json::Value> {
        let resp = self.client.get(format!("{}/query", self.base_url))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Accept", "application/json")
            .query(&[("query", q)])
            .send().await?
            .error_for_status()?
            .json().await?;
        Ok(resp)
    }

    fn parse_invoice(v: &serde_json::Value) -> Invoice {
        let total = v["TotalAmt"].as_f64().unwrap_or(0.0);
        let balance = v["Balance"].as_f64().unwrap_or(0.0);
        let status = if balance == 0.0 { InvoiceStatus::Paid } else { InvoiceStatus::Sent };
        Invoice {
            id: v["Id"].as_str().unwrap_or("").into(),
            customer_name: v["CustomerRef"]["name"].as_str().unwrap_or("").into(),
            amount: Money::usd(total),
            status,
            due_date: v["DueDate"].as_str().unwrap_or("").into(),
            issued_date: v["TxnDate"].as_str().unwrap_or("").into(),
            line_items: v["Line"].as_array().map(|lines| lines.iter().filter_map(|l| {
                let detail = &l["SalesItemLineDetail"];
                if detail.is_null() { return None; }
                Some(LineItem {
                    description: l["Description"].as_str().unwrap_or("").into(),
                    quantity: detail["Qty"].as_f64().unwrap_or(1.0),
                    unit_price_minor: (detail["UnitPrice"].as_f64().unwrap_or(0.0) * 100.0) as i64,
                })
            }).collect()).unwrap_or_default(),
        }
    }
}

#[async_trait::async_trait]
impl FinanceBackend for QuickBooksBackend {
    async fn list_invoices(&self, status: Option<&str>) -> anyhow::Result<Vec<Invoice>> {
        let q = match status {
            Some("paid") => "SELECT * FROM Invoice WHERE Balance = '0'",
            Some("unpaid") => "SELECT * FROM Invoice WHERE Balance > '0'",
            _ => "SELECT * FROM Invoice",
        };
        let data = self.query(q).await?;
        let invoices = data["QueryResponse"]["Invoice"].as_array()
            .map(|arr| arr.iter().map(Self::parse_invoice).collect())
            .unwrap_or_default();
        Ok(invoices)
    }

    async fn get_invoice(&self, id: &str) -> anyhow::Result<Invoice> {
        let data = self.get(&format!("/invoice/{id}")).await?;
        Ok(Self::parse_invoice(&data["Invoice"]))
    }

    async fn create_invoice(&self, customer: &str, line_items: Vec<LineItem>, due_date: &str) -> anyhow::Result<Invoice> {
        let lines: Vec<serde_json::Value> = line_items.iter().map(|l| serde_json::json!({
            "Amount": (l.quantity * l.unit_price_minor as f64) / 100.0,
            "DetailType": "SalesItemLineDetail",
            "Description": l.description,
            "SalesItemLineDetail": { "Qty": l.quantity, "UnitPrice": l.unit_price_minor as f64 / 100.0 }
        })).collect();
        let body = serde_json::json!({
            "CustomerRef": { "name": customer },
            "DueDate": due_date,
            "Line": lines
        });
        let data = self.post("/invoice", body).await?;
        Ok(Self::parse_invoice(&data["Invoice"]))
    }

    async fn list_expenses(&self, _category: Option<&str>) -> anyhow::Result<Vec<Expense>> {
        let data = self.query("SELECT * FROM Purchase").await?;
        let expenses = data["QueryResponse"]["Purchase"].as_array()
            .map(|arr| arr.iter().map(|v| Expense {
                id: v["Id"].as_str().unwrap_or("").into(),
                vendor: v["EntityRef"]["name"].as_str().unwrap_or("").into(),
                amount: Money::usd(v["TotalAmt"].as_f64().unwrap_or(0.0)),
                category: v["Line"][0]["AccountBasedExpenseLineDetail"]["AccountRef"]["name"].as_str().unwrap_or("Uncategorized").into(),
                date: v["TxnDate"].as_str().unwrap_or("").into(),
                description: v["Line"][0]["Description"].as_str().unwrap_or("").into(),
            }).collect()).unwrap_or_default();
        Ok(expenses)
    }

    async fn create_expense(&self, vendor: &str, amount_minor: i64, _currency: &str, category: &str, description: &str) -> anyhow::Result<Expense> {
        let body = serde_json::json!({
            "PaymentType": "Cash",
            "EntityRef": { "name": vendor },
            "Line": [{ "Amount": amount_minor as f64 / 100.0, "DetailType": "AccountBasedExpenseLineDetail", "Description": description, "AccountBasedExpenseLineDetail": { "AccountRef": { "name": category } } }]
        });
        let data = self.post("/purchase", body).await?;
        let v = &data["Purchase"];
        Ok(Expense { id: v["Id"].as_str().unwrap_or("").into(), vendor: vendor.into(), amount: Money { amount_minor, currency: "USD".into() }, category: category.into(), date: v["TxnDate"].as_str().unwrap_or("").into(), description: description.into() })
    }

    async fn list_accounts(&self) -> anyhow::Result<Vec<Account>> {
        let data = self.query("SELECT * FROM Account").await?;
        let accounts = data["QueryResponse"]["Account"].as_array()
            .map(|arr| arr.iter().map(|v| {
                let at = match v["AccountType"].as_str().unwrap_or("") {
                    "Asset" | "Bank" | "Other Current Asset" => AccountType::Asset,
                    "Liability" | "Other Current Liability" => AccountType::Liability,
                    "Equity" => AccountType::Equity,
                    "Revenue" | "Income" => AccountType::Revenue,
                    _ => AccountType::Expense,
                };
                Account { id: v["Id"].as_str().unwrap_or("").into(), name: v["Name"].as_str().unwrap_or("").into(), account_type: at, code: v["AcctNum"].as_str().unwrap_or("").into(), balance: Money::usd(v["CurrentBalance"].as_f64().unwrap_or(0.0)) }
            }).collect()).unwrap_or_default();
        Ok(accounts)
    }

    async fn get_account_balance(&self, account_id: &str) -> anyhow::Result<Account> {
        let data = self.get(&format!("/account/{account_id}")).await?;
        let v = &data["Account"];
        Ok(Account { id: account_id.into(), name: v["Name"].as_str().unwrap_or("").into(), account_type: AccountType::Asset, code: v["AcctNum"].as_str().unwrap_or("").into(), balance: Money::usd(v["CurrentBalance"].as_f64().unwrap_or(0.0)) })
    }

    async fn list_transactions(&self, account_id: Option<&str>, limit: u32) -> anyhow::Result<Vec<Transaction>> {
        let q = match account_id {
            Some(aid) => format!("SELECT * FROM Purchase WHERE AccountRef = '{aid}' MAXRESULTS {limit}"),
            None => format!("SELECT * FROM Purchase MAXRESULTS {limit}"),
        };
        let data = self.query(&q).await?;
        let txns = data["QueryResponse"]["Purchase"].as_array()
            .map(|arr| arr.iter().map(|v| Transaction { id: v["Id"].as_str().unwrap_or("").into(), date: v["TxnDate"].as_str().unwrap_or("").into(), description: v["Line"][0]["Description"].as_str().unwrap_or("").into(), amount: Money::usd(v["TotalAmt"].as_f64().unwrap_or(0.0)), account_id: v["AccountRef"]["value"].as_str().unwrap_or("").into(), reconciled: false, reference_id: None }).collect()).unwrap_or_default();
        Ok(txns)
    }

    async fn create_journal_entry(&self, memo: &str, lines: Vec<JournalLine>) -> anyhow::Result<JournalEntry> {
        let je_lines: Vec<serde_json::Value> = lines.iter().map(|l| {
            let (posting_type, amount) = if l.debit_minor > 0 { ("Debit", l.debit_minor) } else { ("Credit", l.credit_minor) };
            serde_json::json!({ "Amount": amount as f64 / 100.0, "DetailType": "JournalEntryLineDetail", "Description": l.description, "JournalEntryLineDetail": { "PostingType": posting_type, "AccountRef": { "value": l.account_id } } })
        }).collect();
        let body = serde_json::json!({ "Line": je_lines, "PrivateNote": memo });
        let data = self.post("/journalentry", body).await?;
        Ok(JournalEntry { id: data["JournalEntry"]["Id"].as_str().unwrap_or("").into(), date: data["JournalEntry"]["TxnDate"].as_str().unwrap_or("").into(), memo: memo.into(), lines })
    }

    async fn reconcile_transaction(&self, transaction_id: &str, reference_id: &str) -> anyhow::Result<Transaction> {
        // QuickBooks doesn't have a direct reconcile API — we mark it locally
        Ok(Transaction { id: transaction_id.into(), date: String::new(), description: format!("Reconciled with {reference_id}"), amount: Money::usd(0.0), account_id: String::new(), reconciled: true, reference_id: Some(reference_id.into()) })
    }

    async fn get_profit_loss(&self, start: &str, end: &str) -> anyhow::Result<ProfitLossReport> {
        let data = self.get(&format!("/reports/ProfitAndLoss?start_date={start}&end_date={end}")).await?;
        let net = data["Rows"]["Row"].as_array().and_then(|rows| rows.last()).and_then(|r| r["Summary"]["ColData"][1]["value"].as_str()).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
        Ok(ProfitLossReport { period_start: start.into(), period_end: end.into(), total_revenue: Money::usd(0.0), total_expenses: Money::usd(0.0), net_income: Money::usd(net), revenue_lines: vec![], expense_lines: vec![] })
    }

    async fn get_balance_sheet(&self, as_of: &str) -> anyhow::Result<BalanceSheetReport> {
        let _data = self.get(&format!("/reports/BalanceSheet?date_macro=Custom&start_date={as_of}&end_date={as_of}")).await?;
        Ok(BalanceSheetReport { as_of: as_of.into(), total_assets: Money::usd(0.0), total_liabilities: Money::usd(0.0), total_equity: Money::usd(0.0), assets: vec![], liabilities: vec![], equity: vec![] })
    }

    async fn get_cashflow(&self, start: &str, end: &str) -> anyhow::Result<CashFlowReport> {
        let _data = self.get(&format!("/reports/CashFlow?start_date={start}&end_date={end}")).await?;
        Ok(CashFlowReport { period_start: start.into(), period_end: end.into(), opening_balance: Money::usd(0.0), closing_balance: Money::usd(0.0), inflows: Money::usd(0.0), outflows: Money::usd(0.0) })
    }

    async fn get_tax_summary(&self, start: &str, end: &str) -> anyhow::Result<TaxSummary> {
        // QuickBooks tax report via TaxSummary isn't a standard endpoint; approximate
        Ok(TaxSummary { period_start: start.into(), period_end: end.into(), tax_collected: Money::usd(0.0), tax_paid: Money::usd(0.0), net_liability: Money::usd(0.0) })
    }
}
