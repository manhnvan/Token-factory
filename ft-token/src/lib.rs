use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider,
};
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LazyOption;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{env, BorshStorageKey, near_bindgen, AccountId, PanicOnDefault, PromiseOrValue, Balance, Promise};

near_sdk::setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Token,
    Metadata,
}

#[near_bindgen]
impl Contract {
    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// the given fungible token metadata.
    #[init]
    pub fn new(
        owner_id: ValidAccountId,
        total_supply: U128,
        metadata: FungibleTokenMetadata,
    ) -> Self {
        metadata.assert_valid();
        let mut this = Self {
            token: FungibleToken::new(StorageKey::Token),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
        };
        this.token.internal_register_account(owner_id.as_ref());
        this.token.internal_deposit(owner_id.as_ref(), total_supply.into());
        this
    }

    #[payable]
    pub fn mint(&mut self, account_id: ValidAccountId) {
        let amount: Balance = env::attached_deposit();
        self.token.internal_deposit(account_id.as_ref(), amount);
    }

    pub fn withdraw(&mut self, amount: U128) -> Promise {
        let account_id: AccountId = env::predecessor_account_id();
        self.token.internal_withdraw(&account_id, amount.0);
        Promise::new(account_id).transfer(amount.0)
    }
}

near_contract_standards::impl_fungible_token_core!(Contract, token);
near_contract_standards::impl_fungible_token_storage!(Contract, token);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}


#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    use near_sdk::test_utils::{VMContextBuilder, accounts};
    use near_sdk::{testing_env, env, Balance};
    use near_sdk::MockedBlockchain;

    const MINT_STORAGE_COST: u128 = 58700000000000000000000;
    const TOTAL_SUPPLY: Balance = 1_000_000_000_000_000;
    

    fn get_context(is_view: bool) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder.
        current_account_id(accounts(0))
        .signer_account_id(accounts(0))
        .predecessor_account_id(accounts(0))
        .is_view(is_view);
        builder
    }

    fn get_sample_metadata() -> FungibleTokenMetadata {
        FungibleTokenMetadata { 
            spec: "ft-1.0.0".to_string(),
            name: "ManhnvCoin".to_string(),
            symbol: "MNC".to_string(),
            icon: None,
            reference: None,
            reference_hash: None,
            decimals: 1,
        }
    }

    #[test]
    fn test_init_contract() {
        let mut context = get_context(false);
        testing_env!(context.build());
        
        // Init contract
        let metadata = get_sample_metadata();
        let total_supply =  U128::from(587000000000000000000000000);
        let mut contract = Contract::new(accounts(0), total_supply, metadata);

        testing_env!(
            context.storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build()
        );

        let balance = contract.ft_balance_of(accounts(0));
        let total_supply_contract = contract.ft_total_supply();

        assert_eq!(balance.0, total_supply_contract.0);
        assert_eq!(total_supply_contract.0, total_supply.0);
        assert_eq!(balance.0, total_supply.0);
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(false);
        testing_env!(context.build());
        let metadata = get_sample_metadata();
        let total_supply = 1_000_000_000_000_000;
        let mut contract = Contract::new(accounts(0), total_supply.into(), metadata);
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(0))
            .build());
        // Paying for account registration, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(0))
            .build());
        let transfer_amount = total_supply / 3;
        contract.ft_transfer(accounts(1), transfer_amount.into(), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert_eq!(contract.ft_balance_of(accounts(0)).0, (total_supply - transfer_amount));
        assert_eq!(contract.ft_balance_of(accounts(1)).0, transfer_amount);
    }


    #[test]
    fn test_mint() {
        let deposit_amount = 100000;
        let mut context = get_context(false);
        testing_env!(context.build());
        let metadata = get_sample_metadata();
        let total_supply = 1_000_000_000_000_000;
        let mut contract = Contract::new(accounts(0), total_supply.into(), metadata);
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(1))
            .build());
        // Paying for account registration, aka storage deposit
        contract.storage_deposit(None, None);

        // testing_env!(context
        //     .storage_usage(env::storage_usage())
        //     .attached_deposit(deposit_amount)
        //     .predecessor_account_id(accounts(1))
        //     .build());
        // contract.mint(accounts(1));

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(false)
            .predecessor_account_id(accounts(2))
            .attached_deposit(deposit_amount)
            .build());
        contract.mint(accounts(1));

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .predecessor_account_id(accounts(1))
            .build());

        assert_eq!(contract.ft_balance_of(accounts(1)).0, deposit_amount * 2);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(false)
            .predecessor_account_id(accounts(1))
            .build());
        let balance_before = accounts(1);
        contract.withdraw(deposit_amount.into());
        assert_eq!(contract.ft_balance_of(accounts(1)).0, deposit_amount);
    }
}