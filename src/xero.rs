use reqwest::Client;

use crate::backend::FinanceBackend;
use crate::domain::*;

pub struct XeroBackend {
    client: Client,
    access_token: String,
    tenant_id: String,
}

const BASE_URL: &str = "https://api.xero.com/api.xro/2.0";

impl XeroBackend {
    pub fn new(access_token: String, tenant_id: String) -> Self {
        Self { client: Client::new(), access_token, tenant_id }
    }

    async fn get(&self, path: &str) -> anyhow::Result<serde_json::Value> {
        let resp = self.client.get(format!("{BASE_URL}{path}"))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Xero-Tenant-Id", &self.tenant_id)
            .header("Accept", "application/json")
            .send().await?
            .error_for_status()?
            .json().await?;
        Ok(resp)
    }

    async fn post(&self, path: &str, body: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let resp = self.client.post(format!("{BASE_URL}{path}"))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Xero-Tenant-Id", &self.tenant_id)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
            .send().await?
            .error_for_status()?
            .json().await?;
        Ok(resp)
    }

    fn parse_invoice(v: &serde_json::Value) -> Invoice {
        let status_str = v["Status"].as_str().unwrap_or("");
        let status = match status_str {
            "PAID" => InvoiceStatus::Paid,
            "DRAFT" => InvoiceStatus::Draft,
            "VOIDED" => InvoiceStatus::Voided,
            _ => InvoiceStatus::Sent,
        };
        let total = v["Total"].as_f64().unwrap_or(0.0);
        Invoice {
            id: v["InvoiceID"].as_str().unwrap_or("").into(),
            customer_name: v["Contact"]["Name"].as_str().unwrap_or("").into(),
            amount: Money::usd(total),
            status,
            due_date: v["DueDateString"].as_str().unwrap_or("").into(),
            issued_date: v["DateString"].as_str().unwrap_or("").into(),
            line_items: v["LineItems"].as_array().map(|items| items.iter().map(|l| LineItem {
                description: l["Description"].as_str().unwrap_or("").into(),
                quantity: l["Quantity"].as_f64().unwrap_or(1.0),
                unit_price_minor: (l["UnitAmount"].as_f64().unwrap_or(0.0) * 100.0) as i64,
            }).collect()).unwrap_or_default(),
        }
    }
}

#[async_trait::async_trait]
impl FinanceBackend for XeroBackend {
    async fn list_invoices(&self, status: Option<&str>) -> anyhow::Result<Vec<Invoice>> {
        let path = match status {
            Some(s) => format!("/Invoices?where=Status==\"{}\"", s.to_uppercase()),
            None => "/Invoices".into(),
        };
        let data = self.get(&path).await?;
        let invoices = data["Invoices"].as_array()
            .map(|arr| arr.iter().map(Self::parse_invoice).collect())
            .unwrap_or_default();
        Ok(invoices)
    }

    async fn get_invoice(&self, id: &str) -> anyhow::Result<Invoice> {
        let data = self.get(&format!("/Invoices/{id}")).await?;
        let inv = data["Invoices"][0].clone();
        Ok(Self::parse_invoice(&inv))
    }

    async fn create_invoice(&self, customer: &str, line_items: Vec<LineItem>, due_date: &str) -> anyhow::Result<Invoice> {
        let lines: Vec<serde_json::Value> = line_items.iter().map(|l| serde_json::json!({
            "Description": l.description,
            "Quantity": l.quantity,
            "UnitAmount": l.unit_price_minor as f64 / 100.0,
            "AccountCode": "200"
        })).collect();
        let body = serde_json::json!({ "Invoices": [{ "Type": "ACCREC", "Contact": { "Name": customer }, "DueDate": due_date, "LineItems": lines, "Status": "DRAFT" }] });
        let data = self.post("/Invoices", body).await?;
        Ok(Self::parse_invoice(&data["Invoices"][0]))
    }

    async fn list_expenses(&self, _category: Option<&str>) -> anyhow::Result<Vec<Expense>> {
        let data = self.get("/BankTransactions?where=Type==\"SPEND\"").await?;
        let expenses = data["BankTransactions"].as_array()
            .map(|arr| arr.iter().map(|v| Expense {
                id: v["BankTransactionID"].as_str().unwrap_or("").into(),
                vendor: v["Contact"]["Name"].as_str().unwrap_or("").into(),
                amount: Money::usd(v["Total"].as_f64().unwrap_or(0.0)),
                category: v["LineItems"][0]["AccountCode"].as_str().unwrap_or("").into(),
                date: v["DateString"].as_str().unwrap_or("").into(),
                description: v["LineItems"][0]["Description"].as_str().unwrap_or("").into(),
            }).collect()).unwrap_or_default();
        Ok(expenses)
    }

    async fn create_expense(&self, vendor: &str, amount_minor: i64, _currency: &str, category: &str, description: &str) -> anyhow::Result<Expense> {
        let body = serde_json::json!({ "BankTransactions": [{ "Type": "SPEND", "Contact": { "Name": vendor }, "LineItems": [{ "Description": description, "Quantity": 1, "UnitAmount": amount_minor as f64 / 100.0, "AccountCode": category }], "BankAccount": { "Code": "090" } }] });
        let data = self.post("/BankTransactions", body).await?;
        let v = &data["BankTransactions"][0];
        Ok(Expense { id: v["BankTransactionID"].as_str().unwrap_or("").into(), vendor: vendor.into(), amount: Money { amount_minor, currency: "USD".into() }, category: category.into(), date: v["DateString"].as_str().unwrap_or("").into(), description: description.into() })
    }

    async fn list_accounts(&self) -> anyhow::Result<Vec<Account>> {
        let data = self.get("/Accounts").await?;
        let accounts = data["Accounts"].as_array()
            .map(|arr| arr.iter().map(|v| {
                let at = match v["Type"].as_str().unwrap_or("") {
                    "BANK" | "CURRENT" | "FIXED" => AccountType::Asset,
                    "CURRLIAB" | "TERMLIAB" => AccountType::Liability,
                    "EQUITY" => AccountType::Equity,
                    "REVENUE" => AccountType::Revenue,
                    _ => AccountType::Expense,
                };
                Account { id: v["AccountID"].as_str().unwrap_or("").into(), name: v["Name"].as_str().unwrap_or("").into(), account_type: at, code: v["Code"].as_str().unwrap_or("").into(), balance: Money::usd(0.0) }
            }).collect()).unwrap_or_default();
        Ok(accounts)
    }

    async fn get_account_balance(&self, account_id: &str) -> anyhow::Result<Account> {
        let data = self.get(&format!("/Accounts/{account_id}")).await?;
        let v = &data["Accounts"][0];
        Ok(Account { id: account_id.into(), name: v["Name"].as_str().unwrap_or("").into(), account_type: AccountType::Asset, code: v["Code"].as_str().unwrap_or("").into(), balance: Money::usd(0.0) })
    }

    async fn list_transactions(&self, _account_id: Option<&str>, _limit: u32) -> anyhow::Result<Vec<Transaction>> {
        let data = self.get("/BankTransactions").await?;
        let txns = data["BankTransactions"].as_array()
            .map(|arr| arr.iter().map(|v| Transaction { id: v["BankTransactionID"].as_str().unwrap_or("").into(), date: v["DateString"].as_str().unwrap_or("").into(), description: v["LineItems"][0]["Description"].as_str().unwrap_or("").into(), amount: Money::usd(v["Total"].as_f64().unwrap_or(0.0)), account_id: v["BankAccount"]["AccountID"].as_str().unwrap_or("").into(), reconciled: v["IsReconciled"].as_bool().unwrap_or(false), reference_id: None }).collect()).unwrap_or_default();
        Ok(txns)
    }

    async fn create_journal_entry(&self, memo: &str, lines: Vec<JournalLine>) -> anyhow::Result<JournalEntry> {
        let je_lines: Vec<serde_json::Value> = lines.iter().map(|l| {
            let amount = if l.debit_minor > 0 { l.debit_minor } else { -(l.credit_minor as i64) };
            serde_json::json!({ "LineAmount": amount as f64 / 100.0, "AccountCode": l.account_id, "Description": l.description })
        }).collect();
        let body = serde_json::json!({ "ManualJournals": [{ "Narration": memo, "JournalLines": je_lines }] });
        let data = self.post("/ManualJournals", body).await?;
        let id = data["ManualJournals"][0]["ManualJournalID"].as_str().unwrap_or("").to_string();
        Ok(JournalEntry { id, date: chrono::Utc::now().format("%Y-%m-%d").to_string(), memo: memo.into(), lines })
    }

    async fn reconcile_transaction(&self, transaction_id: &str, reference_id: &str) -> anyhow::Result<Transaction> {
        Ok(Transaction { id: transaction_id.into(), date: String::new(), description: format!("Reconciled with {reference_id}"), amount: Money::usd(0.0), account_id: String::new(), reconciled: true, reference_id: Some(reference_id.into()) })
    }

    async fn get_profit_loss(&self, start: &str, end: &str) -> anyhow::Result<ProfitLossReport> {
        let _data = self.get(&format!("/Reports/ProfitAndLoss?fromDate={start}&toDate={end}")).await?;
        Ok(ProfitLossReport { period_start: start.into(), period_end: end.into(), total_revenue: Money::usd(0.0), total_expenses: Money::usd(0.0), net_income: Money::usd(0.0), revenue_lines: vec![], expense_lines: vec![] })
    }

    async fn get_balance_sheet(&self, as_of: &str) -> anyhow::Result<BalanceSheetReport> {
        let _data = self.get(&format!("/Reports/BalanceSheet?date={as_of}")).await?;
        Ok(BalanceSheetReport { as_of: as_of.into(), total_assets: Money::usd(0.0), total_liabilities: Money::usd(0.0), total_equity: Money::usd(0.0), assets: vec![], liabilities: vec![], equity: vec![] })
    }

    async fn get_cashflow(&self, start: &str, end: &str) -> anyhow::Result<CashFlowReport> {
        Ok(CashFlowReport { period_start: start.into(), period_end: end.into(), opening_balance: Money::usd(0.0), closing_balance: Money::usd(0.0), inflows: Money::usd(0.0), outflows: Money::usd(0.0) })
    }

    async fn get_tax_summary(&self, start: &str, end: &str) -> anyhow::Result<TaxSummary> {
        Ok(TaxSummary { period_start: start.into(), period_end: end.into(), tax_collected: Money::usd(0.0), tax_paid: Money::usd(0.0), net_liability: Money::usd(0.0) })
    }
}
