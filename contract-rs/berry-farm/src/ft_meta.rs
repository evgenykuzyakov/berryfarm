use crate::*;
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};

const CUCUMBER_SVG: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' width='256' height='256'%3E%3Cdefs%3E%3Ctext id='A' x='50' y='180' font-size='180'%3E🥒%3C/text%3E%3C/defs%3E%3Cuse xlink:href='%23A'/%3E%3C/svg%3E%0A";

#[near_bindgen]
impl FungibleTokenMetadataProvider for Farm {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: String::from("Cucumber"),
            symbol: String::from("CUKE"),
            icon: Some(String::from(CUCUMBER_SVG)),
            reference: None,
            reference_hash: None,
            decimals: 18,
        }
    }
}
