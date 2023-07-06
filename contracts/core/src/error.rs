use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid Stable Denom")]
    InvalidStable {},

    #[error("Invalid Denom")]
    InvalidDenom {},

    #[error("Pairs that already exist")]
    AlreadyExistsPair {},
    #[error("Missing Pool Contarct")]
    MissingPoolContractAddr {},
    #[error("Pool Contract Instantiate Failed")]
    PoolContractInstantiationFailed {},
    #[error("Treasury Contract Instantiate Failed")]
    TreasuryContractInstantiationFailed {},
    #[error("Invalid Reply ID")]
    InvalidReplyId {},
}
