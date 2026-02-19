#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec};

/// Basic skeleton for a revenue-share contract.
///
/// This is intentionally minimal and focuses on the high-level shape:
/// - Registering a startup "offering"
/// - Recording a revenue report
/// - Emitting events that an off-chain distribution engine can consume

#[contract]
pub struct RevoraRevenueShare;

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Offering {
    pub issuer: Address,
    pub token: Address,
    pub revenue_share_bps: u32,
}

/// Storage keys for offering persistence.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Total number of offerings registered by an issuer.
    OfferCount(Address),
    /// Individual offering stored at (issuer, index).
    OfferItem(Address, u32),
}

/// Maximum number of offerings returned in a single page.
const MAX_PAGE_LIMIT: u32 = 20;

const EVENT_REVENUE_REPORTED: Symbol = symbol_short!("rev_rep");

#[contractimpl]
impl RevoraRevenueShare {
    /// Register a new revenue-share offering.
    /// In a production contract this would handle access control, supply caps,
    /// and issuance hooks. Here we only emit an event.
    pub fn register_offering(env: Env, issuer: Address, token: Address, revenue_share_bps: u32) {
        issuer.require_auth();

        // Persist the offering with an auto-incrementing index.
        let count_key = DataKey::OfferCount(issuer.clone());
        let count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);

        let offering = Offering {
            issuer: issuer.clone(),
            token: token.clone(),
            revenue_share_bps,
        };

        let item_key = DataKey::OfferItem(issuer.clone(), count);
        env.storage().persistent().set(&item_key, &offering);
        env.storage().persistent().set(&count_key, &(count + 1));

        env.events().publish(
            (symbol_short!("offer_reg"), issuer.clone()),
            (token, revenue_share_bps),
        );
    }

    /// Record a revenue report for an offering.
    /// The actual payout calculation and distribution can be performed either
    /// fully on-chain or in a hybrid model where this event is the trigger.
    pub fn report_revenue(env: Env, issuer: Address, token: Address, amount: i128, period_id: u64) {
        issuer.require_auth();

        env.events().publish(
            (EVENT_REVENUE_REPORTED, issuer.clone(), token.clone()),
            (amount, period_id),
        );
    }
    /// Return the total number of offerings registered by `issuer`.
    pub fn get_offering_count(env: Env, issuer: Address) -> u32 {
        let count_key = DataKey::OfferCount(issuer);
        env.storage().persistent().get(&count_key).unwrap_or(0)
    }

    /// Return a page of offerings for `issuer`.
    ///
    /// # Arguments
    /// * `start` – Zero-based cursor indicating where to begin reading.
    /// * `limit` – Maximum items to return. Capped at `MAX_PAGE_LIMIT` (20).
    ///
    /// # Returns
    /// A tuple of `(offerings, next_cursor)` where `next_cursor` is `None`
    /// when there are no more items after this page.
    pub fn get_offerings_page(
        env: Env,
        issuer: Address,
        start: u32,
        limit: u32,
    ) -> (Vec<Offering>, Option<u32>) {
        let count: u32 = Self::get_offering_count(env.clone(), issuer.clone());

        // Clamp limit to MAX_PAGE_LIMIT; treat 0 as "use default max".
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        // If start is beyond the total count, return empty.
        if start >= count {
            return (Vec::new(&env), None);
        }

        let end = core::cmp::min(start + effective_limit, count);
        let mut results = Vec::new(&env);

        for i in start..end {
            let item_key = DataKey::OfferItem(issuer.clone(), i);
            let offering: Offering = env.storage().persistent().get(&item_key).unwrap();
            results.push_back(offering);
        }

        let next_cursor = if end < count { Some(end) } else { None };

        (results, next_cursor)
    }
}

mod test;

