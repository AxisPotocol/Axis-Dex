use axis_protocol::es_axis::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_schema::write_api;
fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
    }
}
