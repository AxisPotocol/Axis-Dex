use cosmwasm_schema::cw_serde;

#[cw_serde]
pub enum Position {
    Long,
    Short,
}

impl Position {
    pub fn new(position: bool) -> Self {
        match position {
            true => Position::Long,
            false => Position::Short,
        }
    }
    pub fn convert_boolean(&self) -> bool {
        match self {
            Position::Long => true,
            Position::Short => false,
        }
    }
}
