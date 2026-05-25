use rmcp::schemars;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Money {
    /// Amount in minor units (cents)
    pub amount_minor: i64,
    /// ISO 4217 currency code
    pub currency: String,
}

impl Money {
    pub fn usd(dollars: f64) -> Self {
        Self { amount_minor: (dollars * 100.0) as i64, currency: "USD".into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Sent,
    Paid,
    Overdue,
    Voided,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Invoice {
    pub id: String,
    pub customer_name: String,
    pub amount: Money,
    pub status: InvoiceStatus,
    pub due_date: String,
    pub issued_date: String,
    pub line_items: Vec<LineItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct LineItem {
    pub description: String,
    pub quantity: f64,
    pub unit_price_minor: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Expense {
    pub id: String,
    pub vendor: String,
    pub amount: Money,
    pub category: String,
    pub date: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub account_type: AccountType,
    pub code: String,
    pub balance: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Transaction {
    pub id: String,
    pub date: String,
    pub description: String,
    pub amount: Money,
    pub account_id: String,
    pub reconciled: bool,
    pub reference_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct JournalLine {
    pub account_id: String,
    pub debit_minor: i64,
    pub credit_minor: i64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct JournalEntry {
    pub id: String,
    pub date: String,
    pub memo: String,
    pub lines: Vec<JournalLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ProfitLossReport {
    pub period_start: String,
    pub period_end: String,
    pub total_revenue: Money,
    pub total_expenses: Money,
    pub net_income: Money,
    pub revenue_lines: Vec<ReportLine>,
    pub expense_lines: Vec<ReportLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ReportLine {
    pub account_name: String,
    pub amount: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct BalanceSheetReport {
    pub as_of: String,
    pub total_assets: Money,
    pub total_liabilities: Money,
    pub total_equity: Money,
    pub assets: Vec<ReportLine>,
    pub liabilities: Vec<ReportLine>,
    pub equity: Vec<ReportLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CashFlowReport {
    pub period_start: String,
    pub period_end: String,
    pub opening_balance: Money,
    pub closing_balance: Money,
    pub inflows: Money,
    pub outflows: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TaxSummary {
    pub period_start: String,
    pub period_end: String,
    pub tax_collected: Money,
    pub tax_paid: Money,
    pub net_liability: Money,
}
