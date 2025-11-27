#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, String, Symbol, Vec,
};

const TRADE_NS: Symbol = symbol_short!("STRADE");

#[contracttype]
#[derive(Clone)]
pub enum ListingStatus {
    Open,
    Locked,
    Completed,
    Cancelled,
}

#[contracttype]
#[derive(Clone)]
pub struct Listing {
    pub listing_id: u64,
    pub seller: Address,
    pub asset_desc: String,
    pub ask_price: i128,
    pub status: ListingStatus,
    pub best_bid: i128,
    pub best_bidder: Option<Address>,
}

#[contracttype]
#[derive(Clone)]
pub struct Bid {
    pub bidder: Address,
    pub amount: i128,
}

#[contract]
pub struct SafeTradeAuction;

#[contractimpl]
impl SafeTradeAuction {
    // Seller creates a listing with an ask price
    pub fn create_listing(
        env: Env,
        listing_id: u64,
        seller: Address,
        asset_desc: String,
        ask_price: i128,
    ) {
        if ask_price <= 0 {
            panic!("ask_price must be positive");
        }

        let inst = env.storage().instance();
        let l_key = Self::listing_key(listing_id);

        if inst.has(&l_key) {
            panic!("listing exists");
        }

        let listing = Listing {
            listing_id,
            seller,
            asset_desc,
            ask_price,
            status: ListingStatus::Open,
            best_bid: 0,
            best_bidder: None,
        };

        let b_key = Self::bids_key(listing_id);
        let empty: Vec<Bid> = Vec::new(&env);

        inst.set(&l_key, &listing);
        inst.set(&b_key, &empty);
    }

    // Buyer places a bid above current best and ask_price floor
    pub fn place_bid(env: Env, listing_id: u64, bidder: Address, amount: i128) {
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let inst = env.storage().instance();
        let l_key = Self::listing_key(listing_id);
        let b_key = Self::bids_key(listing_id);

        let mut listing: Listing =
            inst.get(&l_key).unwrap_or_else(|| panic!("listing not found"));

        if let ListingStatus::Open = listing.status {
        } else {
            panic!("listing not open");
        }

        if amount < listing.ask_price || amount <= listing.best_bid {
            panic!("bid too low");
        }

        let mut bids: Vec<Bid> =
            inst.get(&b_key).unwrap_or_else(|| panic!("bids missing"));

        let bid = Bid { bidder: bidder.clone(), amount };
        bids.push_back(bid);

        listing.best_bid = amount;
        listing.best_bidder = Some(bidder);

        inst.set(&l_key, &listing);
        inst.set(&b_key, &bids);
    }

    // Seller locks the best bid to proceed with off-chain settlement
    pub fn lock_trade(env: Env, listing_id: u64, caller: Address) {
        let inst = env.storage().instance();
        let l_key = Self::listing_key(listing_id);

        let mut listing: Listing =
            inst.get(&l_key).unwrap_or_else(|| panic!("listing not found"));

        if caller != listing.seller {
            panic!("only seller can lock");
        }

        if let ListingStatus::Open = listing.status {
        } else {
            panic!("must be open");
        }

        if listing.best_bid <= 0 || listing.best_bidder.is_none() {
            panic!("no valid bid");
        }

        listing.status = ListingStatus::Locked;
        inst.set(&l_key, &listing);
    }

    // After off-chain payment/delivery, seller marks trade completed
    pub fn complete_trade(env: Env, listing_id: u64, caller: Address) {
        let inst = env.storage().instance();
        let l_key = Self::listing_key(listing_id);

        let mut listing: Listing =
            inst.get(&l_key).unwrap_or_else(|| panic!("listing not found"));

        if caller != listing.seller {
            panic!("only seller can complete");
        }

        if let ListingStatus::Locked = listing.status {
        } else {
            panic!("must be locked");
        }

        listing.status = ListingStatus::Completed;
        inst.set(&l_key, &listing);
    }

    // Seller cancels listing while still open
    pub fn cancel_listing(env: Env, listing_id: u64, caller: Address) {
        let inst = env.storage().instance();
        let l_key = Self::listing_key(listing_id);

        let mut listing: Listing =
            inst.get(&l_key).unwrap_or_else(|| panic!("listing not found"));

        if caller != listing.seller {
            panic!("only seller can cancel");
        }

        if let ListingStatus::Open = listing.status {
        } else {
            panic!("can cancel only when open");
        }

        listing.status = ListingStatus::Cancelled;
        inst.set(&l_key, &listing);
    }

    // View functions
    pub fn get_listing(env: Env, listing_id: u64) -> Option<Listing> {
        let inst = env.storage().instance();
        let key = Self::listing_key(listing_id);
        inst.get(&key)
    }

    pub fn get_bids(env: Env, listing_id: u64) -> Option<Vec<Bid>> {
        let inst = env.storage().instance();
        let key = Self::bids_key(listing_id);
        inst.get(&key)
    }

    fn listing_key(id: u64) -> (Symbol, u64) {
        (TRADE_NS, id)
    }

    fn bids_key(id: u64) -> (Symbol, Symbol, u64) {
        (TRADE_NS, symbol_short!("BIDS"), id)
    }
}
