use std::sync::Arc;

use adk_mcp_sdk::{HealthCheck, HealthStatus};
use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};

use crate::backend::FinanceBackend;
use crate::domain::*;

// --- Tool input types ---

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListInvoicesInput {
    /// Filter by status: paid, unpaid, overdue, draft (optional)
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetInvoiceInput {
    /// Invoice ID
    pub id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateInvoiceInput {
    /// Customer name
    pub customer_name: String,
    /// Due date (YYYY-MM-DD)
    pub due_date: String,
    /// Line items
    pub line_items: Vec<LineItemInput>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LineItemInput {
    pub description: String,
    pub quantity: f64,
    /// Unit price in minor units (cents)
    pub unit_price_minor: i64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListExpensesInput {
    /// Filter by category (optional)
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateExpenseInput {
    /// Vendor name
    pub vendor: String,
    /// Amount in minor units (cents)
    pub amount_minor: i64,
    /// Currency code (default: USD)
    #[serde(default = "default_currency")]
    pub currency: String,
    /// Expense category
    pub category: String,
    /// Description
    pub description: String,
}

fn default_currency() -> String { "USD".into() }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetAccountBalanceInput {
    /// Account ID
    pub account_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListTransactionsInput {
    /// Filter by account ID (optional)
    #[serde(default)]
    pub account_id: Option<String>,
    /// Max results (default: 50)
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 { 50 }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateJournalEntryInput {
    /// Memo/description for the journal entry
    pub memo: String,
    /// Journal lines (debits must equal credits)
    pub lines: Vec<JournalLineInput>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct JournalLineInput {
    /// Account ID
    pub account_id: String,
    /// Debit amount in minor units
    #[serde(default)]
    pub debit_minor: i64,
    /// Credit amount in minor units
    #[serde(default)]
    pub credit_minor: i64,
    /// Line description
    pub description: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReconcileTransactionInput {
    /// Transaction ID to reconcile
    pub transaction_id: String,
    /// Reference ID (invoice or expense ID to match)
    pub reference_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PeriodInput {
    /// Start date (YYYY-MM-DD)
    pub start_date: String,
    /// End date (YYYY-MM-DD)
    pub end_date: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BalanceSheetInput {
    /// As-of date (YYYY-MM-DD)
    pub as_of: String,
}

// --- Server ---

#[derive(Clone)]
pub struct FinanceServer {
    pub backend: Arc<dyn FinanceBackend>,
}

#[tool_router(server_handler)]
impl FinanceServer {
    #[tool(description = "List invoices with optional status filter (paid, unpaid, overdue, draft)")]
    async fn list_invoices(&self, Parameters(input): Parameters<ListInvoicesInput>) -> String {
        match self.backend.list_invoices(input.status.as_deref()).await {
            Ok(invoices) => serde_json::to_string_pretty(&invoices).unwrap(),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Get invoice details by ID")]
    async fn get_invoice(&self, Parameters(input): Parameters<GetInvoiceInput>) -> String {
        match self.backend.get_invoice(&input.id).await {
            Ok(inv) => serde_json::to_string_pretty(&inv).unwrap(),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Create a new invoice")]
    async fn create_invoice(&self, Parameters(input): Parameters<CreateInvoiceInput>) -> String {
        let items: Vec<LineItem> = input.line_items.into_iter().map(|l| LineItem { description: l.description, quantity: l.quantity, unit_price_minor: l.unit_price_minor }).collect();
        match self.backend.create_invoice(&input.customer_name, items, &input.due_date).await {
            Ok(inv) => serde_json::to_string_pretty(&inv).unwrap(),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List expenses/bills with optional category filter")]
    async fn list_expenses(&self, Parameters(input): Parameters<ListExpensesInput>) -> String {
        match self.backend.list_expenses(input.category.as_deref()).await {
            Ok(expenses) => serde_json::to_string_pretty(&expenses).unwrap(),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Record a new expense")]
    async fn create_expense(&self, Parameters(input): Parameters<CreateExpenseInput>) -> String {
        match self.backend.create_expense(&input.vendor, input.amount_minor, &input.currency, &input.category, &input.description).await {
            Ok(exp) => serde_json::to_string_pretty(&exp).unwrap(),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List chart of accounts")]
    async fn list_accounts(&self) -> String {
        match self.backend.list_accounts().await {
            Ok(accounts) => serde_json::to_string_pretty(&accounts).unwrap(),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Get account balance by account ID")]
    async fn get_account_balance(&self, Parameters(input): Parameters<GetAccountBalanceInput>) -> String {
        match self.backend.get_account_balance(&input.account_id).await {
            Ok(acc) => serde_json::to_string_pretty(&acc).unwrap(),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List recent transactions with optional account filter")]
    async fn list_transactions(&self, Parameters(input): Parameters<ListTransactionsInput>) -> String {
        match self.backend.list_transactions(input.account_id.as_deref(), input.limit).await {
            Ok(txns) => serde_json::to_string_pretty(&txns).unwrap(),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Create a journal entry (debit/credit). Debits must equal credits.")]
    async fn create_journal_entry(&self, Parameters(input): Parameters<CreateJournalEntryInput>) -> String {
        let lines: Vec<JournalLine> = input.lines.into_iter().map(|l| JournalLine { account_id: l.account_id, debit_minor: l.debit_minor, credit_minor: l.credit_minor, description: l.description }).collect();
        match self.backend.create_journal_entry(&input.memo, lines).await {
            Ok(je) => serde_json::to_string_pretty(&je).unwrap(),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Reconcile a transaction by matching it to an invoice or expense")]
    async fn reconcile_transaction(&self, Parameters(input): Parameters<ReconcileTransactionInput>) -> String {
        match self.backend.reconcile_transaction(&input.transaction_id, &input.reference_id).await {
            Ok(txn) => serde_json::to_string_pretty(&txn).unwrap(),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Get Profit & Loss report for a date range")]
    async fn get_profit_loss(&self, Parameters(input): Parameters<PeriodInput>) -> String {
        match self.backend.get_profit_loss(&input.start_date, &input.end_date).await {
            Ok(report) => serde_json::to_string_pretty(&report).unwrap(),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Get balance sheet as of a specific date")]
    async fn get_balance_sheet(&self, Parameters(input): Parameters<BalanceSheetInput>) -> String {
        match self.backend.get_balance_sheet(&input.as_of).await {
            Ok(report) => serde_json::to_string_pretty(&report).unwrap(),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Get cash flow summary for a date range")]
    async fn get_cashflow(&self, Parameters(input): Parameters<PeriodInput>) -> String {
        match self.backend.get_cashflow(&input.start_date, &input.end_date).await {
            Ok(report) => serde_json::to_string_pretty(&report).unwrap(),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Get tax liability summary for a date range")]
    async fn get_tax_summary(&self, Parameters(input): Parameters<PeriodInput>) -> String {
        match self.backend.get_tax_summary(&input.start_date, &input.end_date).await {
            Ok(report) => serde_json::to_string_pretty(&report).unwrap(),
            Err(e) => format!("Error: {e}"),
        }
    }
}

#[async_trait::async_trait]
impl HealthCheck for FinanceServer {
    async fn check_health(&self) -> HealthStatus {
        // Verify backend is responsive by listing accounts
        match self.backend.list_accounts().await {
            Ok(_) => HealthStatus { healthy: true, message: Some("Backend operational".into()), latency_ms: Some(1) },
            Err(e) => HealthStatus { healthy: false, message: Some(format!("Backend error: {e}")), latency_ms: None },
        }
    }
}
