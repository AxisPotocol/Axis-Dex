use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid Epoch")]
    InvalidEpoch {},
    #[error("Invalid Price Denom")]
    InvalidPrice {},

    #[error("Invalid Denom")]
    InvalidDenom {},

    #[error("Pairs that already exist")]
    AlreadyExistsPair {},
    #[error("Missing Pool Contarct")]
    MissingPoolContractAddr {},
    #[error("Pool Contract Instantiate Failed")]
    PoolContractInstantiationFailed {},
    #[error("Axis Contract Instantiate Failed")]
    AxisContractInstantiationFailed {},
    #[error("Missing Axis Contarct")]
    MissingAxisContractAddr {},
    #[error("Invalid Reply ID")]
    InvalidReplyId {},
}
