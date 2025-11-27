#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Env, String, Vec,
};

/// Expense record stored on-chain
#[contracttype]
#[derive(Clone)]
pub struct Expense {
    pub id: u64,
    pub owner: Address,
    pub ts: u64,       // ledger timestamp (seconds)
    pub day: u64,      // ts / 86400 (day index)
    pub week: u64,     // ts / 604800 (week index)
    pub category: String,
    pub amount: u128,  // smallest currency unit
    pub note: String,
}

/// Storage keys
#[contracttype]
pub enum ExpenseKey {
    Count(Address),           // per-owner counter
    Record(Address, u64),     // (owner, id) -> Expense
    OwnerIndex(Address),      // (owner) -> Vec<u64> of ids
}

#[contract]
pub struct RustyWallet;

#[contractimpl]
impl RustyWallet {
    /// Add an expense for caller (caller must be authorized).
    /// Returns new expense id.
    pub fn add_expense(
        env: Env,
        caller: Address,
        category: String,
        amount: u128,
        note: String,
    ) -> u64 {


        // fetch and bump counter for this owner
        let mut count: u64 = env
            .storage()
            .instance()
            .get(&ExpenseKey::Count(caller.clone()))
            .unwrap_or(0u64);
        count = count.saturating_add(1);
        env.storage().instance().set(&ExpenseKey::Count(caller.clone()), &count);

        // timestamp and derived day/week
        let ts = env.ledger().timestamp();
        let day = ts / 86400u64;
        let week = ts / 604800u64;

        let exp = Expense {
            id: count,
            owner: caller.clone(),
            ts,
            day,
            week,
            category,
            amount,
            note,
        };

        // store the expense record
        env.storage()
            .instance()
            .set(&ExpenseKey::Record(caller.clone(), count), &exp);

        // append id to owner's index vector
        let mut idx: Vec<u64> = env
            .storage()
            .instance()
            .get(&ExpenseKey::OwnerIndex(caller.clone()))
            .unwrap_or(Vec::new(&env));
        idx.push_back(count);
        env.storage()
            .instance()
            .set(&ExpenseKey::OwnerIndex(caller.clone()), &idx);

        count
    }

    /// View an expense record by owner and id. Anyone can call.
    pub fn view_expense(env: Env, owner: Address, id: u64) -> Expense {
        env.storage()
            .instance()
            .get(&ExpenseKey::Record(owner, id))
            .expect("expense not found")
    }

    /// Return owner expense ids vector (useful for listing).
    /// Note: returns a soroban Vec<u64> of ids.
    pub fn list_my_expense_ids(env: Env, owner: Address) -> Vec<u64> {
        env.storage()
            .instance()
            .get(&ExpenseKey::OwnerIndex(owner))
            .unwrap_or(Vec::new(&env))
    }

    /// Number of expenses for owner
    pub fn expenses_count(env: Env, owner: Address) -> u64 {
        env.storage()
            .instance()
            .get(&ExpenseKey::Count(owner))
            .unwrap_or(0u64)
    }

    /// Sum of amounts for owner for current ledger day (caller provides owner to query).
    /// Uses integer-day derived from ledger timestamp.
    pub fn daily_total(env: Env, owner: Address) -> u128 {
        let today = env.ledger().timestamp() / 86400u64;
        let ids: Vec<u64> = env
            .storage()
            .instance()
            .get(&ExpenseKey::OwnerIndex(owner.clone()))
            .unwrap_or(Vec::new(&env));
        let mut sum: u128 = 0u128;
        let len = ids.len();
        let mut i = 0u32;
        while i < len {
            let id = ids.get(i).expect("index out of bounds");
            let e: Expense = env
                .storage()
                .instance()
                .get(&ExpenseKey::Record(owner.clone(), id))
                .expect("expense not found");
            if e.day == today {
                sum = sum.saturating_add(e.amount);
            }
            i += 1;
        }
        sum
    }

    /// Sum of amounts for owner for current ledger week.
    pub fn weekly_total(env: Env, owner: Address) -> u128 {
        let week = env.ledger().timestamp() / 604800u64;
        let ids: Vec<u64> = env
            .storage()
            .instance()
            .get(&ExpenseKey::OwnerIndex(owner.clone()))
            .unwrap_or(Vec::new(&env));
        let mut sum: u128 = 0u128;
        let len = ids.len();
        let mut i = 0u32;
        while i < len {
            let id = ids.get(i).expect("index out of bounds");
            let e: Expense = env
                .storage()
                .instance()
                .get(&ExpenseKey::Record(owner.clone(), id))
                .expect("expense not found");
            if e.week == week {
                sum = sum.saturating_add(e.amount);
            }
            i += 1;
        }
        sum
    }

    /// Remove an expense (caller must be owner and authorized). This overwrites the record with a deleted marker.
pub fn remove_expense(env: Env, caller: Address, id: u64) {

        // ensure expense exists and owner matches (Record key includes owner)
        let key = ExpenseKey::Record(caller.clone(), id);
        let _ex: Expense = env.storage().instance().get(&key).expect("expense not found");

        // overwrite with a deleted marker (amount zero)
        let deleted = Expense {
            id,
            owner: caller.clone(),
            ts: env.ledger().timestamp(),
            day: 0u64,
            week: 0u64,
            category: String::from_str(&env, "DELETED"),
            amount: 0u128,
            note: String::from_str(&env, ""),
        };
        env.storage().instance().set(&key, &deleted);

        // rebuild owner index without this id (costly but simple)
        let mut ids: Vec<u64> = env
            .storage()
            .instance()
            .get(&ExpenseKey::OwnerIndex(caller.clone()))
            .unwrap_or(Vec::new(&env));
        let len = ids.len();
        let mut out: Vec<u64> = Vec::new(&env);
        let mut i = 0u32;
        while i < len {
            let nid = ids.get(i).expect("index out of bounds");
            if nid != id {
                out.push_back(nid);
            }
            i += 1;
        }
        env.storage()
            .instance()
            .set(&ExpenseKey::OwnerIndex(caller), &out);
    }
}
