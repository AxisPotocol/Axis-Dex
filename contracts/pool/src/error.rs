use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Leverage amount must be less than pool 10%")]
    OverflowLeverage {},

    #[error("InvalidDenom")]
    InvalidDenom {},

    #[error("DivideByZeroError")]
    DivisionError {},

    #[error("Convert Error")]
    ConvertError {},

    #[error("Invalid Allowance")]
    InvalidLPAllowance {},

    #[error("Invalid Amount")]
    InvalidAmount {},

    #[error("Invalid ReplyId")]
    InvalidReplyId,

    #[error("Missing Market Contract Addr")]
    MissingMarketContractAddr {},

    #[error("Market Contract Instantiation Failed")]
    MarketContractInstantiationFailed {},

    #[error("Pool is Lock")]
    PoolLock {},
}
