use crate::domain::*;

#[async_trait::async_trait]
pub trait FinanceBackend: Send + Sync {
    async fn list_invoices(&self, status: Option<&str>) -> anyhow::Result<Vec<Invoice>>;
    async fn get_invoice(&self, id: &str) -> anyhow::Result<Invoice>;
    async fn create_invoice(&self, customer: &str, line_items: Vec<LineItem>, due_date: &str) -> anyhow::Result<Invoice>;
    async fn list_expenses(&self, category: Option<&str>) -> anyhow::Result<Vec<Expense>>;
    async fn create_expense(&self, vendor: &str, amount_minor: i64, currency: &str, category: &str, description: &str) -> anyhow::Result<Expense>;
    async fn list_accounts(&self) -> anyhow::Result<Vec<Account>>;
    async fn get_account_balance(&self, account_id: &str) -> anyhow::Result<Account>;
    async fn list_transactions(&self, account_id: Option<&str>, limit: u32) -> anyhow::Result<Vec<Transaction>>;
    async fn create_journal_entry(&self, memo: &str, lines: Vec<JournalLine>) -> anyhow::Result<JournalEntry>;
    async fn reconcile_transaction(&self, transaction_id: &str, reference_id: &str) -> anyhow::Result<Transaction>;
    async fn get_profit_loss(&self, start: &str, end: &str) -> anyhow::Result<ProfitLossReport>;
    async fn get_balance_sheet(&self, as_of: &str) -> anyhow::Result<BalanceSheetReport>;
    async fn get_cashflow(&self, start: &str, end: &str) -> anyhow::Result<CashFlowReport>;
    async fn get_tax_summary(&self, start: &str, end: &str) -> anyhow::Result<TaxSummary>;
}
