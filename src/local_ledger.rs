use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

use crate::backend::FinanceBackend;
use crate::domain::*;

pub struct LocalLedger {
    invoices: Mutex<HashMap<String, Invoice>>,
    expenses: Mutex<HashMap<String, Expense>>,
    accounts: Mutex<HashMap<String, Account>>,
    transactions: Mutex<HashMap<String, Transaction>>,
    journals: Mutex<HashMap<String, JournalEntry>>,
}

impl LocalLedger {
    pub fn new() -> Self {
        let ledger = Self {
            invoices: Mutex::new(HashMap::new()),
            expenses: Mutex::new(HashMap::new()),
            accounts: Mutex::new(HashMap::new()),
            transactions: Mutex::new(HashMap::new()),
            journals: Mutex::new(HashMap::new()),
        };
        ledger.seed();
        ledger
    }

    fn seed(&self) {
        let accounts = vec![
            Account { id: "acc-1".into(), name: "Cash".into(), account_type: AccountType::Asset, code: "1000".into(), balance: Money::usd(50000.0) },
            Account { id: "acc-2".into(), name: "Accounts Receivable".into(), account_type: AccountType::Asset, code: "1100".into(), balance: Money::usd(12500.0) },
            Account { id: "acc-3".into(), name: "Revenue".into(), account_type: AccountType::Revenue, code: "4000".into(), balance: Money::usd(75000.0) },
            Account { id: "acc-4".into(), name: "Office Expenses".into(), account_type: AccountType::Expense, code: "5000".into(), balance: Money::usd(8200.0) },
            Account { id: "acc-5".into(), name: "Accounts Payable".into(), account_type: AccountType::Liability, code: "2000".into(), balance: Money::usd(3400.0) },
        ];
        for a in accounts {
            self.accounts.lock().unwrap().insert(a.id.clone(), a);
        }

        let invoices = vec![
            Invoice { id: "inv-1".into(), customer_name: "Acme Corp".into(), amount: Money::usd(5000.0), status: InvoiceStatus::Paid, due_date: "2026-04-30".into(), issued_date: "2026-04-01".into(), line_items: vec![LineItem { description: "Consulting".into(), quantity: 10.0, unit_price_minor: 50000 }] },
            Invoice { id: "inv-2".into(), customer_name: "Globex Inc".into(), amount: Money::usd(7500.0), status: InvoiceStatus::Sent, due_date: "2026-06-15".into(), issued_date: "2026-05-01".into(), line_items: vec![LineItem { description: "Software License".into(), quantity: 1.0, unit_price_minor: 750000 }] },
            Invoice { id: "inv-3".into(), customer_name: "Initech".into(), amount: Money::usd(2200.0), status: InvoiceStatus::Overdue, due_date: "2026-05-01".into(), issued_date: "2026-04-15".into(), line_items: vec![LineItem { description: "Support Hours".into(), quantity: 22.0, unit_price_minor: 10000 }] },
        ];
        for i in invoices {
            self.invoices.lock().unwrap().insert(i.id.clone(), i);
        }

        let expenses = vec![
            Expense { id: "exp-1".into(), vendor: "Office Depot".into(), amount: Money::usd(450.0), category: "Office Supplies".into(), date: "2026-05-10".into(), description: "Printer paper and toner".into() },
            Expense { id: "exp-2".into(), vendor: "AWS".into(), amount: Money::usd(1200.0), category: "Cloud Services".into(), date: "2026-05-01".into(), description: "Monthly infrastructure".into() },
        ];
        for e in expenses {
            self.expenses.lock().unwrap().insert(e.id.clone(), e);
        }

        let txns = vec![
            Transaction { id: "txn-1".into(), date: "2026-05-01".into(), description: "Invoice payment from Acme".into(), amount: Money::usd(5000.0), account_id: "acc-1".into(), reconciled: true, reference_id: Some("inv-1".into()) },
            Transaction { id: "txn-2".into(), date: "2026-05-10".into(), description: "Office supplies purchase".into(), amount: Money { amount_minor: -45000, currency: "USD".into() }, account_id: "acc-1".into(), reconciled: false, reference_id: None },
        ];
        for t in txns {
            self.transactions.lock().unwrap().insert(t.id.clone(), t);
        }
    }
}

#[async_trait::async_trait]
impl FinanceBackend for LocalLedger {
    async fn list_invoices(&self, status: Option<&str>) -> anyhow::Result<Vec<Invoice>> {
        let map = self.invoices.lock().unwrap();
        let iter = map.values();
        let result: Vec<Invoice> = match status {
            Some(s) => iter.filter(|i| serde_json::to_string(&i.status).unwrap_or_default().contains(s)).cloned().collect(),
            None => iter.cloned().collect(),
        };
        Ok(result)
    }

    async fn get_invoice(&self, id: &str) -> anyhow::Result<Invoice> {
        self.invoices.lock().unwrap().get(id).cloned().ok_or_else(|| anyhow::anyhow!("Invoice not found: {id}"))
    }

    async fn create_invoice(&self, customer: &str, line_items: Vec<LineItem>, due_date: &str) -> anyhow::Result<Invoice> {
        let total: i64 = line_items.iter().map(|l| (l.quantity * l.unit_price_minor as f64) as i64).sum();
        let inv = Invoice {
            id: format!("inv-{}", &Uuid::new_v4().to_string()[..8]),
            customer_name: customer.into(),
            amount: Money { amount_minor: total, currency: "USD".into() },
            status: InvoiceStatus::Draft,
            due_date: due_date.into(),
            issued_date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            line_items,
        };
        self.invoices.lock().unwrap().insert(inv.id.clone(), inv.clone());
        Ok(inv)
    }

    async fn list_expenses(&self, category: Option<&str>) -> anyhow::Result<Vec<Expense>> {
        let map = self.expenses.lock().unwrap();
        let result: Vec<Expense> = match category {
            Some(c) => map.values().filter(|e| e.category.to_lowercase().contains(&c.to_lowercase())).cloned().collect(),
            None => map.values().cloned().collect(),
        };
        Ok(result)
    }

    async fn create_expense(&self, vendor: &str, amount_minor: i64, currency: &str, category: &str, description: &str) -> anyhow::Result<Expense> {
        let exp = Expense {
            id: format!("exp-{}", &Uuid::new_v4().to_string()[..8]),
            vendor: vendor.into(),
            amount: Money { amount_minor, currency: currency.into() },
            category: category.into(),
            date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            description: description.into(),
        };
        self.expenses.lock().unwrap().insert(exp.id.clone(), exp.clone());
        Ok(exp)
    }

    async fn list_accounts(&self) -> anyhow::Result<Vec<Account>> {
        Ok(self.accounts.lock().unwrap().values().cloned().collect())
    }

    async fn get_account_balance(&self, account_id: &str) -> anyhow::Result<Account> {
        self.accounts.lock().unwrap().get(account_id).cloned().ok_or_else(|| anyhow::anyhow!("Account not found: {account_id}"))
    }

    async fn list_transactions(&self, account_id: Option<&str>, limit: u32) -> anyhow::Result<Vec<Transaction>> {
        let map = self.transactions.lock().unwrap();
        let mut txns: Vec<Transaction> = match account_id {
            Some(aid) => map.values().filter(|t| t.account_id == aid).cloned().collect(),
            None => map.values().cloned().collect(),
        };
        txns.sort_by(|a, b| b.date.cmp(&a.date));
        txns.truncate(limit as usize);
        Ok(txns)
    }

    async fn create_journal_entry(&self, memo: &str, lines: Vec<JournalLine>) -> anyhow::Result<JournalEntry> {
        let total_debits: i64 = lines.iter().map(|l| l.debit_minor).sum();
        let total_credits: i64 = lines.iter().map(|l| l.credit_minor).sum();
        if total_debits != total_credits {
            anyhow::bail!("Debits ({total_debits}) must equal credits ({total_credits})");
        }
        let entry = JournalEntry {
            id: format!("je-{}", &Uuid::new_v4().to_string()[..8]),
            date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            memo: memo.into(),
            lines,
        };
        self.journals.lock().unwrap().insert(entry.id.clone(), entry.clone());
        Ok(entry)
    }

    async fn reconcile_transaction(&self, transaction_id: &str, reference_id: &str) -> anyhow::Result<Transaction> {
        let mut map = self.transactions.lock().unwrap();
        let txn = map.get_mut(transaction_id).ok_or_else(|| anyhow::anyhow!("Transaction not found: {transaction_id}"))?;
        txn.reconciled = true;
        txn.reference_id = Some(reference_id.into());
        Ok(txn.clone())
    }

    async fn get_profit_loss(&self, start: &str, end: &str) -> anyhow::Result<ProfitLossReport> {
        let accounts = self.accounts.lock().unwrap();
        let revenue_lines: Vec<ReportLine> = accounts.values()
            .filter(|a| matches!(a.account_type, AccountType::Revenue))
            .map(|a| ReportLine { account_name: a.name.clone(), amount: a.balance.clone() })
            .collect();
        let expense_lines: Vec<ReportLine> = accounts.values()
            .filter(|a| matches!(a.account_type, AccountType::Expense))
            .map(|a| ReportLine { account_name: a.name.clone(), amount: a.balance.clone() })
            .collect();
        let total_rev: i64 = revenue_lines.iter().map(|l| l.amount.amount_minor).sum();
        let total_exp: i64 = expense_lines.iter().map(|l| l.amount.amount_minor).sum();
        Ok(ProfitLossReport {
            period_start: start.into(),
            period_end: end.into(),
            total_revenue: Money { amount_minor: total_rev, currency: "USD".into() },
            total_expenses: Money { amount_minor: total_exp, currency: "USD".into() },
            net_income: Money { amount_minor: total_rev - total_exp, currency: "USD".into() },
            revenue_lines,
            expense_lines,
        })
    }

    async fn get_balance_sheet(&self, as_of: &str) -> anyhow::Result<BalanceSheetReport> {
        let accounts = self.accounts.lock().unwrap();
        let assets: Vec<ReportLine> = accounts.values().filter(|a| matches!(a.account_type, AccountType::Asset)).map(|a| ReportLine { account_name: a.name.clone(), amount: a.balance.clone() }).collect();
        let liabilities: Vec<ReportLine> = accounts.values().filter(|a| matches!(a.account_type, AccountType::Liability)).map(|a| ReportLine { account_name: a.name.clone(), amount: a.balance.clone() }).collect();
        let equity: Vec<ReportLine> = accounts.values().filter(|a| matches!(a.account_type, AccountType::Equity)).map(|a| ReportLine { account_name: a.name.clone(), amount: a.balance.clone() }).collect();
        Ok(BalanceSheetReport {
            as_of: as_of.into(),
            total_assets: Money { amount_minor: assets.iter().map(|l| l.amount.amount_minor).sum(), currency: "USD".into() },
            total_liabilities: Money { amount_minor: liabilities.iter().map(|l| l.amount.amount_minor).sum(), currency: "USD".into() },
            total_equity: Money { amount_minor: equity.iter().map(|l| l.amount.amount_minor).sum(), currency: "USD".into() },
            assets,
            liabilities,
            equity,
        })
    }

    async fn get_cashflow(&self, start: &str, end: &str) -> anyhow::Result<CashFlowReport> {
        let txns = self.transactions.lock().unwrap();
        let inflows: i64 = txns.values().filter(|t| t.amount.amount_minor > 0).map(|t| t.amount.amount_minor).sum();
        let outflows: i64 = txns.values().filter(|t| t.amount.amount_minor < 0).map(|t| t.amount.amount_minor.abs()).sum();
        let opening = 5000000i64; // $50,000
        Ok(CashFlowReport {
            period_start: start.into(),
            period_end: end.into(),
            opening_balance: Money { amount_minor: opening, currency: "USD".into() },
            closing_balance: Money { amount_minor: opening + inflows - outflows, currency: "USD".into() },
            inflows: Money { amount_minor: inflows, currency: "USD".into() },
            outflows: Money { amount_minor: outflows, currency: "USD".into() },
        })
    }

    async fn get_tax_summary(&self, start: &str, end: &str) -> anyhow::Result<TaxSummary> {
        // Simplified: 10% of revenue collected, expenses have embedded tax
        let accounts = self.accounts.lock().unwrap();
        let revenue: i64 = accounts.values().filter(|a| matches!(a.account_type, AccountType::Revenue)).map(|a| a.balance.amount_minor).sum();
        let collected = revenue / 10;
        let paid = 15000i64; // $150 in tax paid on expenses
        Ok(TaxSummary {
            period_start: start.into(),
            period_end: end.into(),
            tax_collected: Money { amount_minor: collected, currency: "USD".into() },
            tax_paid: Money { amount_minor: paid, currency: "USD".into() },
            net_liability: Money { amount_minor: collected - paid, currency: "USD".into() },
        })
    }
}
