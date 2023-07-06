use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Overflow Error")]
    OverflowError {},
    #[error("InvalidPosition")]
    InvalidPosition {},

    #[error("OverFlowMaxLeverage")]
    OverFlowMaxLeverage {},

    #[error("InvalidLeverage")]
    InvalidLeverage {},

    #[error("InvalidFunds")]
    InvalidCollateral {},

    #[error("ZeroFunds")]
    ZeroFunds {},

    #[error("LowFunds")]
    LowFunds {},

    #[error("InvalidDenom")]
    InvalidDenom {},

    #[error("Not Found ExchangeRate")]
    NotFoundExchangeRate {},

    #[error("Leverage Amount is bigger than Pool balance")]
    LeverageAmountBigerThanPoolBalance {},

    #[error("Oracle Price Deciaml Error")]
    DecimalError {},

    #[error("User Only One Position")]
    TraderOnlyOnePosition {},

    #[error("Parse Error")]
    ParseError {},

    #[error("Convert Error")]
    ConvertError {},
}
